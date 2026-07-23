use std::sync::Arc;

use flume::Sender;
use log::{error, info};
use once_cell::sync::Lazy;
use tokio::task::JoinSet;
use ytpapi2::{Endpoint, YoutubeMusicInstance};

use crate::{
    consts::CONFIG,
    run_service,
    structures::performance,
    term::{ManagerMessage, Screens},
    AUTH_TOKEN,
};

pub fn spawn_api_task(updater_s: Sender<ManagerMessage>) {
    run_service(async move {
        info!("API task on");
        let guard = performance::guard("API task");

        let api = match create_api_instance().await {
            Some(api) => api,
            None => {
                info!("No API instance created, running without backend features");
                return;
            }
        };

        let api = Arc::new(api);
        let mut set = JoinSet::new();

        let api_ = api.clone();
        let updater_s_ = updater_s.clone();
        set.spawn(async move {
            match api_.get_home(2).await {
                Ok(e) => {
                    for playlist in e.playlists {
                        spawn_browse_playlist_task(
                            playlist.clone(),
                            api_.clone(),
                            updater_s_.clone(),
                        )
                    }
                }
                Err(e) => error!("get_home {e:?}"),
            }
        });

        if AUTH_TOKEN.read().unwrap().is_some() {
            let api_ = api.clone();
            let updater_s_ = updater_s.clone();
            set.spawn(async move {
                match api_.get_library(&Endpoint::MusicLikedPlaylists, 2).await {
                    Ok(e) => {
                        for playlist in e {
                            spawn_browse_playlist_task(
                                playlist.clone(),
                                api_.clone(),
                                updater_s_.clone(),
                            )
                        }
                    }
                    Err(e) => error!("MusicLikedPlaylists -> {e:?}"),
                }
            });

            let api_ = api.clone();
            let updater_s_ = updater_s.clone();
            set.spawn(async move {
                match api_.get_library(&Endpoint::MusicLibraryLanding, 2).await {
                    Ok(e) => {
                        for playlist in e {
                            spawn_browse_playlist_task(
                                playlist.clone(),
                                api_.clone(),
                                updater_s_.clone(),
                            )
                        }
                    }
                    Err(e) => error!("MusicLibraryLanding -> {e:?}"),
                }
            });
        }

        while let Some(e) = set.join_next().await {
            e.unwrap();
        }

        drop(guard);
    });
}

async fn create_api_instance() -> Option<YoutubeMusicInstance> {
    let token = AUTH_TOKEN.read().unwrap().clone();
    if let Some(token) = token {
        if !token.access_token.is_empty() {
            info!("Using OAuth token for API instance");
            match YoutubeMusicInstance::new_oauth(token.access_token).await {
                Ok(instance) => return Some(instance),
                Err(e) => error!("Failed to create OAuth API instance: {e:?}"),
            }
        }
    }
    info!("Creating anonymous API instance");
    match YoutubeMusicInstance::new_anonymous().await {
        Ok(instance) => Some(instance),
        Err(e) => {
            error!("Failed to create anonymous API instance: {e:?}");
            None
        }
    }
}

static BROWSED_PLAYLISTS: Lazy<Mutex<Vec<(String, String)>>> = Lazy::new(|| Mutex::new(vec![]));

use std::sync::Mutex;

fn spawn_browse_playlist_task(
    playlist: ytpapi2::YoutubeMusicPlaylistRef,
    api: Arc<YoutubeMusicInstance>,
    updater_s: Sender<ManagerMessage>,
) {
    if playlist.browse_id.starts_with("UC") && CONFIG.player.hide_channels_on_homepage {
        log::info!(
            "Skipping channel (CONFIG) {} {}",
            playlist.name,
            playlist.browse_id
        );
        return;
    }
    if playlist.browse_id.starts_with("MPREb_") && CONFIG.player.hide_albums_on_homepage {
        log::info!(
            "Skipping album (CONFIG) {} {}",
            playlist.name,
            playlist.browse_id
        );
        return;
    }
    {
        let mut k = BROWSED_PLAYLISTS.lock().unwrap();
        if k.iter()
            .any(|(name, id)| name == &playlist.name && id == &playlist.browse_id)
        {
            return;
        }
        k.push((playlist.name.clone(), playlist.browse_id.clone()));
    }

    run_service(async move {
        let guard = format!("Browse playlist {} {}", playlist.name, playlist.browse_id);
        let guard = performance::guard(&guard);
        match api.get_playlist(&playlist, 5).await {
            Ok(videos) => {
                if videos.len() < 2 {
                    info!("Playlist {} is too small so skipped", playlist.name);
                    return;
                }
                let _ = updater_s.send(
                    ManagerMessage::AddElementToChooser((
                        format!("{} ({})", playlist.name, playlist.subtitle),
                        videos,
                    ))
                    .pass_to(Screens::Playlist),
                );
            }
            Err(e) => {
                error!("{e:?}");
            }
        }
        drop(guard);
    });
}
