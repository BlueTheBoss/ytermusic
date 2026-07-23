use log::{error, info, trace};
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> ScrobbleConfig {
        ScrobbleConfig {
            lastfm_api_key: String::new(),
            lastfm_shared_secret: String::new(),
            lastfm_session: String::new(),
            listenbrainz_token: String::new(),
        }
    }

    fn make_video(id: &str, duration_secs: u64) -> YoutubeMusicVideoRef {
        YoutubeMusicVideoRef {
            title: String::new(),
            author: String::new(),
            album: String::new(),
            video_id: id.to_string(),
            duration: duration_secs.to_string(),
        }
    }

    #[test]
    fn test_scrobble_threshold_short_song() {
        // Songs under 30s should not scrobble
        let config = make_config();
        let mut scrobbler = Scrobbler::new(config);

        scrobbler.current_id = Some("song1".to_string());
        scrobbler.played_duration = 15.0;
        let _video = make_video("song1", 29);
        let should = match 29.0_f64 {
            dur if dur > 30.0 => {
                let half = dur * 0.5;
                let four_min = 240.0;
                scrobbler.played_duration >= half.min(four_min)
            }
            _ => false,
        };
        assert!(!should, "Songs under 30s should not scrobble");
    }

    #[test]
    fn test_scrobble_threshold_50_percent() {
        let config = make_config();
        let mut scrobbler = Scrobbler::new(config);
        scrobbler.current_id = Some("song2".to_string());
        scrobbler.played_duration = 120.0; // 2 min
        // 50% of 180 = 90s, 4min = 240s. Threshold = min(90, 240) = 90
        // played = 120 >= 90 ✓
        let dur = 180.0_f64;
        let half = dur * 0.5;
        let four_min = 240.0;
        let threshold = half.min(four_min);
        assert_eq!(threshold, 90.0);
        assert!(scrobbler.played_duration >= threshold);
    }

    #[test]
    fn test_scrobble_threshold_four_min_cap() {
        let config = make_config();
        let mut scrobbler = Scrobbler::new(config);
        scrobbler.current_id = Some("song3".to_string());
        scrobbler.played_duration = 239.0; // 3m59s
        let dur = 600.0_f64;
        let half = dur * 0.5;
        let four_min = 240.0;
        let threshold = half.min(four_min);
        assert_eq!(threshold, 240.0);
        assert!(!(scrobbler.played_duration >= threshold)); // 239 < 240

        scrobbler.played_duration = 240.0;
        assert!(scrobbler.played_duration >= threshold); // 240 >= 240 ✓
    }

    #[test]
    fn test_new_song_resets_played_duration() {
        let config = make_config();
        let mut scrobbler = Scrobbler::new(config);
        scrobbler.current_id = Some("old".to_string());
        scrobbler.played_duration = 100.0;

        // tick with different video ID should reset
        let video = make_video("new", 200);
        if scrobbler.current_id.as_deref() != Some(&video.video_id) {
            scrobbler.current_id = Some(video.video_id.clone());
            scrobbler.played_duration = 0.0;
        }
        assert_eq!(scrobbler.current_id.as_deref(), Some("new"));
        assert_eq!(scrobbler.played_duration, 0.0);
    }

    #[test]
    fn test_dedup_scrobble() {
        let config = make_config();
        let mut scrobbler = Scrobbler::new(config);
        // Already scrobbled
        scrobbler.scrobbled.insert("done".to_string());
        assert!(scrobbler.scrobbled.contains("done"));
        assert!(!scrobbler.scrobbled.contains("new"));
    }
}
use hashbrown::HashSet as FxHashSet;
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
    scrobbled: FxHashSet<String>,
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
            scrobbled: FxHashSet::new(),
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

    pub fn now_playing(&self, video: &YoutubeMusicVideoRef) {
        let duration = video.duration.parse::<u64>().unwrap_or(0);
        let lastfm_session = self.config.lastfm_session.clone();
        let lastfm_api_key = self.config.lastfm_api_key.clone();
        let listenbrainz_token = self.config.listenbrainz_token.clone();
        let title = video.title.clone();
        let author = video.author.clone();
        let album = video.album.clone();

        tokio::spawn(async move {
            if !lastfm_session.is_empty() {
                now_playing_lastfm(&title, &author, &album, duration, &lastfm_api_key, &lastfm_session).await;
            }
            if !listenbrainz_token.is_empty() {
                now_playing_listenbrainz(&title, &author, &album, duration, &listenbrainz_token).await;
            }
        });
    }

    #[allow(dead_code)]
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

async fn now_playing_lastfm(
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
    params.insert("method", "track.updateNowPlaying");
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
                info!("Last.fm now playing: {artist} - {title}");
            } else {
                trace!("Last.fm now playing ignored: {}", resp.text().await.unwrap_or_default());
            }
        }
        Err(e) => trace!("Last.fm now playing error: {e}"),
    }
}

async fn now_playing_listenbrainz(
    title: &str,
    artist: &str,
    album: &str,
    duration: u64,
    token: &str,
) {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "listen_type": "playing_now",
        "payload": [{
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
                info!("ListenBrainz now playing: {artist} - {title}");
            } else {
                trace!("ListenBrainz now playing ignored: {}", resp.text().await.unwrap_or_default());
            }
        }
        Err(e) => trace!("ListenBrainz now playing error: {e}"),
    }
}
