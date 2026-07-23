use flume::Sender;
use log::info;
use rand::seq::SliceRandom;
use tokio::task::spawn_blocking;
use ytpapi2::YoutubeMusicVideoRef;

use crate::{
    consts::{CACHE_DIR, CONFIG},
    run_service,
    structures::performance,
    term::{ManagerMessage, Screens},
    DATABASE,
};

pub fn spawn_local_musics_task(updater_s: Sender<ManagerMessage>) {
    run_service(async move {
        info!("Database getter task on");
        let guard = performance::guard("Local musics");
        if let Some(videos) = DATABASE.read() {
            shuffle_and_send(videos, &updater_s);
        } else {
            let mut videos = Vec::new();
            let cache_dir = CACHE_DIR.join("downloads");
            if let Ok(Ok(entries)) = spawn_blocking(move || std::fs::read_dir(cache_dir)).await {
                let mut entries = entries;
                while let Some(Ok(files)) = entries.next() {
                    let path = files.path();
                    if path.as_os_str().to_string_lossy().ends_with(".json") {
                        let path_clone = path.clone();
                        if let Ok(Ok(content)) = spawn_blocking(move || std::fs::read_to_string(path_clone)).await {
                            if let Ok(video) = serde_json::from_str::<YoutubeMusicVideoRef>(&content) {
                                videos.push(video);
                            }
                        }
                    }
                }
            }
            shuffle_and_send(videos, &updater_s);

            DATABASE.write();
        }
        drop(guard);
    });
}

fn shuffle_and_send(mut videos: Vec<YoutubeMusicVideoRef>, updater_s: &Sender<ManagerMessage>) {
    DATABASE.clone_from(&videos);

    if CONFIG.player.shuffle {
        videos.shuffle(&mut rand::thread_rng());
    }

    let _ = updater_s.send(
        ManagerMessage::AddElementToChooser(("Local musics".to_owned(), videos))
            .pass_to(Screens::Playlist),
    );
}
