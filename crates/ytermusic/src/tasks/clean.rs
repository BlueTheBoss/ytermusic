use crate::{consts::CACHE_DIR, run_service, structures::performance};

use tokio::task::spawn_blocking;

/// This function is called on start to clean the database and the files
/// that are incompletely downloaded due to a crash.
pub fn spawn_clean_task() {
    run_service(async move {
        let guard = performance::guard("Clean task");
        let downloads_dir = CACHE_DIR.join("downloads");
        if let Ok(entries) = spawn_blocking(move || {
            std::fs::read_dir(downloads_dir)
        }).await {
            for entry in entries.into_iter().flatten().flatten() {
                let path = entry.path();
                if path.extension().unwrap_or_default() == "mp4" {
                    let mut path1 = path.clone();
                    path1.set_extension("json");
                    if !path1.exists() {
                        let mp4_path = path.clone();
                        let _ = spawn_blocking(move || {
                            let _ = std::fs::remove_file(&mp4_path);
                        }).await;
                    }
                }
            }
        }
        drop(guard);
    });
}
