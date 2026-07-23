use log::{error, info};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use ytpapi2::YoutubeMusicVideoRef;

#[derive(Clone, Debug)]
pub struct ScrobbleConfig {
    pub lastfm_api_key: String,
    pub lastfm_shared_secret: String,
    pub lastfm_session: String,
    pub listenbrainz_token: String,
}

pub struct Scrobbler {
    config: ScrobbleConfig,
    scrobbled: HashSet<String>,
    current_id: Option<String>,
    played_duration: f64,
}

impl Scrobbler {
    pub fn new(config: ScrobbleConfig) -> Self {
        let has_lastfm = !config.lastfm_api_key.is_empty()
            && !config.lastfm_shared_secret.is_empty()
            && !config.lastfm_session.is_empty();
        let has_listenbrainz = !config.listenbrainz_token.is_empty();
        if has_lastfm {
            info!("Scrobbler: Last.fm configured");
        }
        if has_listenbrainz {
            info!("Scrobbler: ListenBrainz configured");
        }
        if !has_lastfm && !has_listenbrainz {
            info!("Scrobbler: no services configured");
        }
        Self {
            config,
            scrobbled: HashSet::new(),
            current_id: None,
            played_duration: 0.0,
        }
    }

    pub fn tick(
        &mut self,
        video: Option<&YoutubeMusicVideoRef>,
        elapsed: Duration,
        duration: Option<f64>,
    ) {
        let video = match video {
            Some(v) => v,
            None => {
                self.current_id = None;
                self.played_duration = 0.0;
                return;
            }
        };

        if self.current_id.as_deref() != Some(&video.video_id) {
            self.current_id = Some(video.video_id.clone());
            self.played_duration = 0.0;
        }

        if self.scrobbled.contains(&video.video_id) {
            return;
        }

        self.played_duration = elapsed.as_secs_f64();

        let should_scrobble = match duration {
            Some(dur) if dur > 30.0 => {
                let half = dur * 0.5;
                let four_min = 240.0;
                let threshold = half.min(four_min);
                self.played_duration >= threshold
            }
            Some(_) => false,
            None => false,
        };

        if should_scrobble {
            self.scrobbled.insert(video.video_id.clone());
            let title = video.title.clone();
            let author = video.author.clone();
            let album = video.album.clone();
            let dur = duration.unwrap_or(0.0) as u64;
            let lastfm_session = self.config.lastfm_session.clone();
            let lastfm_api_key = self.config.lastfm_api_key.clone();
            let listenbrainz_token = self.config.listenbrainz_token.clone();

            tokio::spawn(async move {
                if !lastfm_session.is_empty() {
                    scrobble_lastfm(&title, &author, &album, dur, &lastfm_api_key, &lastfm_session).await;
                }
                if !listenbrainz_token.is_empty() {
                    scrobble_listenbrainz(
                        &title,
                        &author,
                        &album,
                        dur,
                        &listenbrainz_token,
                    )
                    .await;
                }
            });
        }
    }

    pub fn clear(&mut self) {
        self.current_id = None;
        self.played_duration = 0.0;
    }
}

async fn scrobble_lastfm(
    title: &str,
    artist: &str,
    album: &str,
    duration: u64,
    api_key: &str,
    session: &str,
) {
    let client = reqwest::Client::new();
    let duration_str = duration.to_string();
    let mut params = HashMap::new();
    params.insert("method", "track.scrobble");
    params.insert("artist", artist);
    params.insert("track", title);
    params.insert("album", album);
    params.insert("duration", &duration_str);
    params.insert("api_key", api_key);
    params.insert("sk", session);
    params.insert("format", "json");

    match client
        .post("https://ws.audioscrobbler.com/2.0/")
        .form(&params)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("Last.fm scrobble: {artist} - {title}");
            } else {
                error!("Last.fm scrobble failed: {}", resp.text().await.unwrap_or_default());
            }
        }
        Err(e) => error!("Last.fm scrobble error: {e}"),
    }
}

async fn scrobble_listenbrainz(
    title: &str,
    artist: &str,
    album: &str,
    duration: u64,
    token: &str,
) {
    let client = reqwest::Client::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let body = serde_json::json!({
        "listen_type": "single",
        "payload": [{
            "listened_at": now,
            "track_metadata": {
                "artist_name": artist,
                "track_name": title,
                "release_name": album,
                "additional_info": {
                    "duration_ms": duration * 1000,
                    "media_player": "YTerMusic",
                }
            }
        }]
    });

    match client
        .post("https://api.listenbrainz.org/1/submit-listens")
        .header("Authorization", format!("Token {token}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                info!("ListenBrainz scrobble: {artist} - {title}");
            } else {
                error!("ListenBrainz scrobble failed: {}", resp.text().await.unwrap_or_default());
            }
        }
        Err(e) => error!("ListenBrainz scrobble error: {e}"),
    }
}
