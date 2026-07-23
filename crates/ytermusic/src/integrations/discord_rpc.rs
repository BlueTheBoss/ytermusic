use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use log::{info, warn};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ytpapi2::YoutubeMusicVideoRef;

pub struct DiscordRPC {
    client: Option<DiscordIpcClient>,
    connected: bool,
    current_id: Option<String>,
}

impl DiscordRPC {
    pub fn new(client_id: &str) -> Self {
        if client_id.is_empty() {
            info!("Discord RPC disabled (no client ID)");
            return Self {
                client: None,
                connected: false,
                current_id: None,
            };
        }
        let client = DiscordIpcClient::new(client_id);
        info!("Discord RPC client created");
        Self {
            client: Some(client),
            connected: false,
            current_id: None,
        }
    }

    pub fn connect(&mut self) {
        let client = match &mut self.client {
            Some(c) => c,
            None => return,
        };
        if self.connected {
            return;
        }
        match client.connect() {
            Ok(_) => {
                info!("Discord RPC connected");
                self.connected = true;
            }
            Err(e) => {
                warn!("Discord RPC connection failed (Discord may not be running): {e}");
            }
        }
    }

    pub fn update(&mut self, video: Option<&YoutubeMusicVideoRef>, is_paused: bool, elapsed: Duration) {
        if !self.connected {
            self.connect();
        }

        let client = match &mut self.client {
            Some(c) => c,
            None => return,
        };

        if !self.connected {
            return;
        }

        let video_id = video.map(|v| v.video_id.clone()).unwrap_or_default();
        if self.current_id.as_deref() == Some(&video_id) && !is_paused {
            return;
        }
        self.current_id = Some(video_id);

        if video.is_none() {
            let _ = client.clear_activity();
            return;
        }

        let video = video.unwrap();
        let start_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let state = if is_paused { "Paused" } else { "Playing" };
        let details = format!("{} - {}", video.author, video.title);

        let mut activity = activity::Activity::new()
            .state(state)
            .details(&details);

        if is_paused {
            let paused_elapsed = elapsed.as_secs() as i64;
            activity = activity.timestamps(activity::Timestamps::new().start(paused_elapsed));
        } else {
            activity = activity.timestamps(activity::Timestamps::new().start(start_time as i64));
        }

        match client.set_activity(activity) {
            Ok(_) => {}
            Err(e) => {
                self.connected = false;
                warn!("Discord RPC activity update failed: {e}");
            }
        }
    }

    pub fn clear(&mut self) {
        self.current_id = None;
        if let Some(ref mut client) = self.client {
            let _ = client.clear_activity();
        }
    }
}

impl Drop for DiscordRPC {
    fn drop(&mut self) {
        if let Some(ref mut client) = self.client {
            let _ = client.close();
        }
    }
}
