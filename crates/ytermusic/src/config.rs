use log::info;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

use crate::utils::get_project_dirs;

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DownloaderConfig {
    #[default]
    Ytdlp,
    RustyYtdl,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct GlobalConfig {
    /// Maximum number of parallel downloads.
    /// If your downloads are failing, try lowering
    /// this.
    /// Default value is 4.
    #[serde(default = "parallel_downloads")]
    pub parallel_downloads: u16,
    #[serde(default)]
    pub downloader: DownloaderConfig,
}

#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct MusicPlayerConfig {
    /// Initial volume of the player, in percent.
    /// Default value is 50, clamped at 100.
    #[serde(default = "default_volume")]
    pub initial_volume: u8,
    #[serde(default = "default_true")]
    pub dbus: bool,
    #[serde(default = "default_true")]
    pub hide_channels_on_homepage: bool,
    #[serde(default = "default_false")]
    pub hide_albums_on_homepage: bool,
    #[serde(default = "enable_volume_slider")]
    pub volume_slider: bool,
    /// Whether to shuffle playlists before playing
    #[serde(default)]
    pub shuffle: bool,
    #[serde(default = "default_paused_style", with = "StyleDef")]
    pub gauge_paused_style: Style,
    #[serde(default = "default_playing_style", with = "StyleDef")]
    pub gauge_playing_style: Style,
    #[serde(default = "default_nomusic_style", with = "StyleDef")]
    pub gauge_nomusic_style: Style,
    #[serde(default = "default_paused_style", with = "StyleDef")]
    pub text_paused_style: Style,
    #[serde(default = "default_playing_style", with = "StyleDef")]
    pub text_playing_style: Style,
    #[serde(default = "default_nomusic_style", with = "StyleDef")]
    pub text_next_style: Style,
    #[serde(default = "default_nomusic_style", with = "StyleDef")]
    pub text_waiting_style: Style,
    #[serde(default = "default_downloading_style", with = "StyleDef")]
    pub text_downloading_style: Style,
    #[serde(default = "default_error_style", with = "StyleDef")]
    pub text_error_style: Style,
    #[serde(default = "default_searching_style", with = "StyleDef")]
    pub text_searching_style: Style,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(remote = "Style")]
struct StyleDef {
    #[serde(default)]
    fg: Option<Color>,
    #[serde(default)]
    bg: Option<Color>,
    #[serde(default = "Modifier::empty")]
    add_modifier: Modifier,
    #[serde(default = "Modifier::empty")]
    sub_modifier: Modifier,
    #[serde(default)]
    underline_color: Option<Color>,
}

impl Default for MusicPlayerConfig {
    fn default() -> Self {
        Self {
            hide_albums_on_homepage: default_false(),
            hide_channels_on_homepage: default_true(),
            dbus: default_true(),
            initial_volume: default_volume(),
            shuffle: Default::default(),
            gauge_paused_style: default_paused_style(),
            gauge_playing_style: default_playing_style(),
            gauge_nomusic_style: default_nomusic_style(),
            text_paused_style: default_paused_style(),
            text_playing_style: default_playing_style(),
            text_next_style: default_nomusic_style(),
            text_waiting_style: default_nomusic_style(),
            text_error_style: default_error_style(),
            text_searching_style: default_searching_style(),
            text_downloading_style: default_downloading_style(),
            volume_slider: enable_volume_slider(),
        }
    }
}

fn default_searching_style() -> Style {
    Style::default().fg(Color::LightCyan)
}

fn default_error_style() -> Style {
    Style::default().fg(Color::Red)
}

fn parallel_downloads() -> u16 {
    4
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn enable_volume_slider() -> bool {
    true
}

fn default_paused_style() -> Style {
    Style::default().fg(Color::Yellow)
}

fn default_playing_style() -> Style {
    Style::default().fg(Color::Green)
}

fn default_nomusic_style() -> Style {
    Style::default().fg(Color::White)
}

fn default_downloading_style() -> Style {
    Style::default().fg(Color::Blue)
}

fn default_volume() -> u8 {
    50
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PlaylistConfig {}

#[derive(Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct SearchConfig {}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
#[derive(Default)]
pub struct DiscordConfig {
    #[serde(default)]
    pub client_id: String,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Keybinds {
    pub play_pause: String,
    pub next: String,
    pub previous: String,
    pub forward: String,
    pub backward: String,
    pub volume_up: String,
    pub volume_down: String,
    pub search: String,
    pub shuffle: String,
    pub remove: String,
    pub queue_view: String,
    pub lyrics: String,
    pub login: String,
    pub cleanup: String,
    pub scroll_up: String,
    pub scroll_down: String,
    pub enter: String,
    pub escape: String,
    pub repeat_mode: String,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            play_pause: " ".to_string(),
            next: ">".to_string(),
            previous: "<".to_string(),
            forward: "l".to_string(),
            backward: "h".to_string(),
            volume_up: "+".to_string(),
            volume_down: "-".to_string(),
            search: "f".to_string(),
            shuffle: "s".to_string(),
            remove: "r".to_string(),
            queue_view: "Q".to_string(),
            lyrics: "y".to_string(),
            login: "L".to_string(),
            cleanup: "C".to_string(),
            scroll_up: "k".to_string(),
            scroll_down: "j".to_string(),
            enter: "enter".to_string(),
            escape: "esc".to_string(),
            repeat_mode: "m".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
#[derive(Default)]
pub struct ScrobbleConfig {
    #[serde(default)]
    pub lastfm_api_key: String,
    #[serde(default)]
    pub lastfm_shared_secret: String,
    #[serde(default)]
    pub lastfm_session: String,
    #[serde(default)]
    pub listenbrainz_token: String,
}


#[allow(unused)]
#[derive(Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub player: MusicPlayerConfig,
    #[serde(default)]
    pub playlist: PlaylistConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub discord: DiscordConfig,
    #[serde(default)]
    pub scrobble: ScrobbleConfig,
    #[serde(default)]
    pub keybinds: Keybinds,
}

impl Config {
    pub fn new() -> Self {
        // TODO handle errors
        let opt = || {
            let project_dirs = get_project_dirs()?;
            let config_path = project_dirs.config_dir().join("config.toml");
            config_path
                .parent()
                .map(|p| std::fs::create_dir_all(p).ok());
            info!("Loading config from {:?}", config_path);
            if !config_path.exists() {
                let default_config = Self::default();
                std::fs::write(
                    project_dirs.config_dir().join("config.toml"),
                    toml::to_string_pretty(&default_config).ok()?,
                )
                .ok()?;
                return Some(default_config);
            }
            let config_string = std::fs::read_to_string(config_path).ok()?;
            let config = toml::from_str::<Self>(&config_string).ok()?;
            std::fs::write(
                project_dirs.config_dir().join("config.applied.toml"),
                toml::to_string_pretty(&config).ok()?,
            )
            .ok()?;
            Some(config)
        };
        opt().unwrap_or_default()
    }
}
