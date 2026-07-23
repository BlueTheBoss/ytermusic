use std::{
    collections::{HashSet, VecDeque},
    sync::atomic::Ordering,
};

use common_structs::MusicDownloadStatus;
use flume::{unbounded, Receiver, Sender};
use hashbrown::HashMap as FxHashMap;
use log::{debug, error, info, trace};
use player::{PlayError, Player, PlayerOptions};

use ytpapi2::YoutubeMusicVideoRef;

use notify_rust::Notification;

use crate::{
    integrations::{discord_rpc::DiscordRPC, scrobbler::{ScrobbleConfig, Scrobbler}},
    lyrics::lrclib::{self, LyricLine},
    structures::sound_action::RepeatMode,
};

use crate::{
    consts::{CACHE_DIR, CONFIG},
    errors::{handle_error},
    structures::{media::Media, sound_action::SoundAction},
    systems::DOWNLOAD_MANAGER,
    term::{list_selector::ListSelector, playlist::PLAYER_RUNNING, ManagerMessage, Screens},
    AUTH_TOKEN, DATABASE,
};

pub struct PlayerState {
    pub goto: Screens,
    pub list: Vec<YoutubeMusicVideoRef>,
    pub current: usize,
    pub rtcurrent: Option<YoutubeMusicVideoRef>,
    pub music_status: FxHashMap<String, MusicDownloadStatus>,
    pub list_selector: ListSelector,
    pub controls: Media,
    pub sink: Player,
    pub updater: Sender<ManagerMessage>,
    pub soundaction_sender: Sender<SoundAction>,
    pub soundaction_receiver: Receiver<SoundAction>,
    pub stream_error_receiver: Receiver<PlayError>,
    pub repeat_mode: RepeatMode,
    pub current_lyrics: Vec<LyricLine>,
    pub discord_rpc: DiscordRPC,
    pub scrobbler: Scrobbler,
    last_notified_id: Option<String>,
    autoplayed_ids: HashSet<String>,
    last_lyrics_id: Option<String>,
    pub fetching_lyrics_id: Option<String>,
    last_download_list: Vec<String>,
}

impl PlayerState {
    fn new(
        soundaction_sender: Sender<SoundAction>,
        soundaction_receiver: Receiver<SoundAction>,
        updater: Sender<ManagerMessage>,
    ) -> Self {
        let (stream_error_sender, stream_error_receiver) = unbounded::<PlayError>();
        let sink = match Player::new(
            stream_error_sender,
            PlayerOptions::new(CONFIG.player.initial_volume),
        ) {
            Ok(s) => s,
            Err(e) => {
                handle_error(&updater, "player creation error", Err::<(), String>(format!("{e:?}")));
                std::process::exit(1);
            }
        };
        Self {
            controls: Media::new(updater.clone(), soundaction_sender.clone()),
            soundaction_receiver,
            list_selector: ListSelector::default(),
            music_status: FxHashMap::new(),
            updater,
            stream_error_receiver,
            soundaction_sender,
            sink,
            goto: Screens::Playlist,
            list: Vec::new(),
            current: 0,
            rtcurrent: None,
            repeat_mode: RepeatMode::None,
            current_lyrics: Vec::new(),
            discord_rpc: DiscordRPC::new(&CONFIG.discord.client_id),
            scrobbler: Scrobbler::new(ScrobbleConfig {
                lastfm_api_key: CONFIG.scrobble.lastfm_api_key.clone(),
                lastfm_shared_secret: CONFIG.scrobble.lastfm_shared_secret.clone(),
                lastfm_session: CONFIG.scrobble.lastfm_session.clone(),
                listenbrainz_token: CONFIG.scrobble.listenbrainz_token.clone(),
            }),
            last_notified_id: None,
            autoplayed_ids: HashSet::new(),
            last_lyrics_id: None,
            fetching_lyrics_id: None,
            last_download_list: Vec::new(),
        }
    }

    pub fn current(&self) -> Option<&YoutubeMusicVideoRef> {
        self.relative_current(0)
    }

    pub fn relative_current(&self, n: isize) -> Option<&YoutubeMusicVideoRef> {
        self.list.get(self.current.saturating_add_signed(n))
    }

    pub fn set_relative_current(&mut self, n: isize) {
        self.current = self.current.saturating_add_signed(n);
    }

    pub fn is_current_download_failed(&self) -> bool {
        self.current()
            .as_ref()
            .map(|x| {
                self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::DownloadFailed)
            })
            .unwrap_or(false)
    }

    pub fn is_current_downloaded(&self) -> bool {
        self.current()
            .as_ref()
            .map(|x| self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::Downloaded))
            .unwrap_or(false)
    }

    pub fn update(&mut self) {
        PLAYER_RUNNING.store(self.current().is_some(), Ordering::SeqCst);
        debug!("Player update - current: {:?}, list_len: {}, status: {:?}", 
            self.current().map(|v| &v.video_id), self.list.len(), 
            self.current().and_then(|v| self.music_status.get(&v.video_id)));
        self.update_controls();
        self.handle_stream_errors();
        if self.current > self.list.len() {
            self.current = self.list.len();
        }
        while let Ok(e) = self.soundaction_receiver.try_recv() {
            debug!("Received sound action: {:?}", e);
            e.apply_sound_action(self);
        }
        if self.is_current_download_failed() {
            SoundAction::Next(1).apply_sound_action(self);
        }
        if self.sink.is_finished() {
            if self.is_current_downloaded() && self.rtcurrent.as_ref() == self.current() {
                match self.repeat_mode {
                    RepeatMode::One => {
                        // replay current song without advancing
                    }
                    RepeatMode::All => {
                        if self.current >= self.list.len().saturating_sub(1) {
                            self.current = 0;
                        } else {
                            self.set_relative_current(1);
                        }
                    }
                    RepeatMode::None => {
                        self.set_relative_current(1);
                    }
                }
            }
            self.handle_stream_errors();
            self.update_controls();
            // If the current song is finished, we play the next one but if the next one has failed to download, we skip it
            // TODO(optimize this)
            while self
                .current()
                .map(|x| {
                    self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::DownloadFailed)
                })
                .unwrap_or(false)
            {
                self.set_relative_current(1);
            }

            if self.is_current_downloaded() {
                trace!("Current song is downloaded, attempting playback");
                if let Some(video) = self.current().cloned() {
                    let k = CACHE_DIR.join(format!("downloads/{}.mp4", video.video_id));
                    trace!("Attempting to play: {:?}", k);
                    if let Err(e) = self.sink.play(k.as_path()) {
                        if matches!(e, PlayError::DecoderError(_)) {
                            // Cleaning the file

                            DATABASE.remove_video(&video);
                            let mp4_path = k.clone();
                            let json_path = CACHE_DIR.join(format!("downloads/{}.json", video.video_id));
                            tokio::task::spawn_blocking(move || {
                                let _ = std::fs::remove_file(&mp4_path);
                                let _ = std::fs::remove_file(&json_path);
                            });
                            self.current = 0;
                            DATABASE.write();
                        } else {
                            let _ = self.updater.send(ManagerMessage::PassTo(
                                    Screens::DeviceLost,
                                    Box::new(ManagerMessage::Error(
                                        format!("{e:?}"),
                                        Box::new(None),
                                    )),
                                ));
                        }
                    }
                }
            }
        } else {
            self.rtcurrent = self.current().cloned();
        }
        let to_download = self
            .list
            .iter()
            .skip(self.current)
            .chain(self.list.iter().take(self.current).rev())
            .filter(|x| {
                self.music_status.get(&x.video_id) == Some(&MusicDownloadStatus::NotDownloaded)
            })
            .take(12)
            .cloned()
            .collect::<VecDeque<_>>();
        trace!("Queuing {} songs for download", to_download.len());
        let new_ids: Vec<String> = to_download.iter().map(|v| v.video_id.clone()).collect();
        if new_ids != self.last_download_list {
            self.last_download_list = new_ids;
            DOWNLOAD_MANAGER.set_download_list(to_download);
            trace!("Updated download manager queue");
        }

        let current_video = self.current().cloned();
        if let Some(ref video) = current_video {
            if self.last_lyrics_id.as_deref() != Some(&video.video_id)
                && self.fetching_lyrics_id.as_deref() != Some(&video.video_id)
            {
                self.last_lyrics_id = Some(video.video_id.clone());
                self.fetching_lyrics_id = Some(video.video_id.clone());
                let updater = self.updater.clone();
                let video_id = video.video_id.clone();
                let author = video.author.clone();
                let title = video.title.clone();
                let album = video.album.clone();
                let duration_str = video.duration.clone();
                tokio::spawn(async move {
                    let duration = duration_str.parse::<f64>().unwrap_or(0.0);
                    let result = lrclib::fetch_lyrics(&author, &title, &album, duration).await;
                    let _ = updater.send(ManagerMessage::LyricsFetchFinished(video_id));
                    if let Ok(lyrics) = result {
                        let _ = updater.send(ManagerMessage::SetLyrics(lyrics));
                    } else if let Err(e) = result {
                        info!("Lyrics fetch failed for {}: {e}", title);
                    }
                });
            }
        }

        let video_clone = self.current().cloned();
        let is_paused = self.sink.is_paused();
        let elapsed = self.sink.elapsed();
        let duration = video_clone
            .as_ref()
            .and_then(|v| v.duration.parse::<f64>().ok());

        if let Some(ref video) = video_clone {
            if self.last_notified_id.as_deref() != Some(&video.video_id) {
                self.last_notified_id = Some(video.video_id.clone());
                let summary = video.title.clone();
                let body = format!("{} — {}", video.author, video.album);
                let _ = Notification::new()
                    .summary(&summary)
                    .body(&body)
                    .appname("YTerMusic")
                    .show();
                self.scrobbler.now_playing(video);
            }

            let remaining = self.list.len().saturating_sub(self.current + 1);
            if remaining <= 3
                && !self.autoplayed_ids.contains(&video.video_id)
                && !self.sink.is_paused()
                && !self.sink.is_finished()
            {
                self.autoplayed_ids.insert(video.video_id.clone());
                let video_id = video.video_id.clone();
                let updater = self.updater.clone();
                tokio::spawn(async move {
                    let api = create_api_for_autoplay().await;
                    let api = match api {
                        Some(a) => a,
                        None => return,
                    };
                    match api.get_watch_playlist(&video_id).await {
                        Ok(videos) if !videos.is_empty() => {
                            let _ = updater.send(ManagerMessage::AutoplayReady(videos));
                        }
                        Ok(_) => {}
                        Err(e) => info!("Autoplay fetch failed: {e:?}"),
                    }
                });
            }
        }

        self.discord_rpc.update(video_clone.as_ref(), is_paused, elapsed);
        self.scrobbler.tick(video_clone.as_ref(), elapsed, duration);
    }

    fn handle_stream_errors(&self) {
        while let Ok(e) = self.stream_error_receiver.try_recv() {
            error!("Stream error: {:?}", e);
            handle_error(&self.updater, "audio device stream error", Err(e));
        }
    }
    fn update_controls(&mut self) {
        let current = self.current().cloned();
        let result = self
            .controls
            .update(current, &self.sink)
            .map_err(|x| format!("{x:?}"));
        handle_error::<String>(&self.updater, "Can't update finished media control", result);
    }
}

async fn create_api_for_autoplay() -> Option<ytpapi2::YoutubeMusicInstance> {
    let token = AUTH_TOKEN.read().ok()?.clone();
    if let Some(token) = token {
        if !token.access_token.is_empty() {
            match ytpapi2::YoutubeMusicInstance::new_oauth(token.access_token).await {
                Ok(api) => return Some(api),
                Err(e) => info!("Autoplay OAuth API failed: {e:?}"),
            }
        }
    }
    match ytpapi2::YoutubeMusicInstance::new_anonymous().await {
        Ok(api) => Some(api),
        Err(e) => {
            info!("Autoplay anonymous API failed: {e:?}");
            None
        }
    }
}

pub fn player_system(updater: Sender<ManagerMessage>) -> (Sender<SoundAction>, PlayerState) {
    let (tx, rx) = flume::unbounded::<SoundAction>();
    (tx.clone(), PlayerState::new(tx, rx, updater))
}
