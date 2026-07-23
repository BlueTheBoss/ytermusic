use consts::{CACHE_DIR, CONFIG, INTRODUCTION};
use flume::{Receiver, Sender};
use log::{error, info};
use once_cell::sync::Lazy;
use structures::performance::STARTUP_TIME;
use term::{login, Manager, ManagerMessage};
use tokio::select;

use std::{future::Future, panic, sync::RwLock};
use systems::{logger::init, player::player_system};

use crate::{
    structures::{media::run_window_handler, sound_action::download_manager_handler},
    systems::DOWNLOAD_MANAGER,
    utils::get_project_dirs,
};

mod config;
mod consts;
mod database;
mod errors;
mod integrations;
mod lyrics;
mod shutdown;
mod structures;
mod systems;
mod term;
mod utils;
pub use shutdown::{is_shutdown_sent, shutdown, ShutdownSignal};
mod tasks;

pub use database::DATABASE;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static AUTH_TOKEN: Lazy<RwLock<Option<ytpapi2::oauth::OAuthToken>>> =
    Lazy::new(|| RwLock::new(None));

fn run_service<T>(future: T) -> tokio::task::JoinHandle<()>
where
    T: Future + Send + 'static,
{
    tokio::task::spawn(async move {
        select! {
            _ = future => {},
            _ = ShutdownSignal => {},
        }
    })
}

fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{}", INTRODUCTION);
                return;
            }
            "--fix-db" => {
                DATABASE.fix_db();
                DATABASE.write();
                println!("[INFO] Database fixed");
                return;
            }
            "--clear-cache" => {
                match std::fs::remove_dir_all(&*CACHE_DIR) {
                    Ok(_) => {
                        println!("[INFO] Cache cleared");
                    }
                    Err(e) => {
                        println!("[ERROR] Can't clear cache: {e}");
                    }
                }
                return;
            }
            e => {
                println!("Unknown argument `{e}`");
                println!("Here are the available arguments:");
                println!(" - --help: Show help");
                println!(" - --fix-db: Fix the database");
                println!(" - --clear-cache: Erase all the files in cache");
                return;
            }
        }
    }
    panic::set_hook(Box::new(|e| {
        println!("{e}");
        error!("{e}");
        shutdown();
    }));
    init().expect("Failed to initialize logger");
    app_start();
}

async fn app_start_main(updater_r: Receiver<ManagerMessage>, updater_s: Sender<ManagerMessage>) {
    STARTUP_TIME.log("Init");

    let _ = std::fs::create_dir_all(CACHE_DIR.join("downloads"));

    if CONFIG.global.downloader == crate::config::DownloaderConfig::Ytdlp {
        match std::process::Command::new("yt-dlp")
            .arg("--version")
            .output()
        {
            Ok(out) if out.status.success() => {
                info!(
                    "yt-dlp found: {}",
                    String::from_utf8_lossy(&out.stdout).trim()
                );
            }
            _ => {
                println!("yt-dlp not found in PATH");
                println!("get it at https://github.com/yt-dlp/yt-dlp or set downloader = \"rusty_ytdl\" in config");
                let _ = std::io::stdin().read_line(&mut String::new());
                return;
            }
        }
    }

    try_load_oauth_token().await;
    let is_authenticated = AUTH_TOKEN.read().unwrap_or_else(|e| e.into_inner()).is_some();

    if is_authenticated {
        info!("OAuth token loaded, using authenticated mode");
    } else {
        info!("No OAuth token found, running in anonymous mode");
    }

    STARTUP_TIME.log("Startup");

    tasks::clean::spawn_clean_task();

    STARTUP_TIME.log("Spawned clean task");
    let (sa, player) = player_system(updater_s.clone());
    DOWNLOAD_MANAGER.spawn_system(ShutdownSignal, download_manager_handler(sa.clone()));
    STARTUP_TIME.log("Spawned system task");
    tasks::last_playlist::spawn_last_playlist_task(updater_s.clone());
    STARTUP_TIME.log("Spawned last playlist task");
    tasks::api::spawn_api_task(updater_s.clone());
    STARTUP_TIME.log("Spawned api task");
    tasks::local_musics::spawn_local_musics_task(updater_s.clone());

    STARTUP_TIME.log("Running manager");
    let mut manager = Manager::new(sa, player).await;
    manager.set_updater(updater_s);
    if let Err(e) = manager.run(&updater_r) {
        error!("Terminal error: {e}");
    }
}

async fn try_load_oauth_token() {
    let token = login::load_token();
    if let Some(mut token) = token {
        if token.refresh_token.is_empty() {
            return;
        }
        if token.expires_at <= std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
        {
            info!("OAuth token expired, refreshing");
            match ytpapi2::oauth::refresh_access_token(&token.refresh_token).await {
                Ok(refreshed) => {
                    token = refreshed;
                    if let Some(dirs) = get_project_dirs() {
                        let path = dirs.config_dir().join("oauth.json");
                        let _ = std::fs::create_dir_all(dirs.config_dir());
                        let _ = std::fs::write(&path, serde_json::to_string_pretty(&token).unwrap_or_default());
                    }
                }
                Err(e) => {
                    error!("Failed to refresh OAuth token: {e:?}");
                    return;
                }
            }
        }
        let mut guard = AUTH_TOKEN.write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(token);
    }
}

fn app_start() {
    let (updater_s, updater_r) = flume::unbounded::<ManagerMessage>();
    let updater_s_c = updater_s.clone();
    ctrlc::set_handler(move || {
        info!("CTRL-C received");
        shutdown()
    })
    .expect("Error setting Ctrl-C handler");
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to build runtime")
            .block_on(async move {
                select! {
                    _ = app_start_main(updater_r, updater_s) => {},
                    _ = ShutdownSignal => {},
                };
            });
        info!("Runtime closed");
    });
    run_window_handler(&updater_s_c);
}
