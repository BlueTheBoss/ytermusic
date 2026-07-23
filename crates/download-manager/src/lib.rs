mod task;

use log::debug;
use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use database::YTLocalDatabase;
use tokio::{select, sync::Notify, task::JoinHandle};
use ytpapi2::YoutubeMusicVideoRef;

use common_structs::MusicDownloadStatus;

pub type MessageHandler = Arc<dyn Fn(DownloadManagerMessage) + Send + Sync + 'static>;

pub enum DownloadManagerMessage {
    VideoStatusUpdate(String, MusicDownloadStatus),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Downloader {
    YtDlp,
    #[cfg(feature = "rusty-ytdl-backend")]
    RustyYtdl,
}

pub struct DownloadManager {
    database: &'static YTLocalDatabase,
    cache_dir: PathBuf,
    parallel_downloads: u16,
    pub downloader: Downloader,
    handles: Mutex<Vec<JoinHandle<()>>>,
    download_list: Mutex<VecDeque<YoutubeMusicVideoRef>>,
    in_download: Mutex<HashSet<String>>,
    notify: Notify,
}

impl DownloadManager {
    pub fn new(
        cache_dir: PathBuf,
        database: &'static YTLocalDatabase,
        parallel_downloads: u16,
        downloader: Downloader,
    ) -> Self {
        Self {
            database,
            cache_dir,
            parallel_downloads,
            downloader,
            handles: Mutex::new(Vec::new()),
            download_list: Mutex::new(VecDeque::new()),
            in_download: Mutex::new(HashSet::new()),
            notify: Notify::new(),
        }
    }

    pub fn remove_from_in_downloads(&self, video: &String) {
        if let Ok(mut guard) = self.in_download.lock() {
            guard.remove(video);
        }
    }

    fn take(&self) -> Option<YoutubeMusicVideoRef> {
        self.download_list.lock().ok()?.pop_front()
    }

    /// This has to be called as a service stream
    /// HANDLES.lock().unwrap().push(run_service(async move {
    ///     run_service_stream(sender);
    /// }));
    pub fn run_service_stream(
        &'static self,
        cancelation: impl Future<Output = ()> + Clone + Send + 'static,
        sender: MessageHandler,
    ) {
        let fut = async move {
            loop {
                if let Some(id) = self.take() {
                    debug!("Starting download for: {}", id);
                    self.start_download(id, sender.clone()).await;
                } else {
                    debug!("No downloads in queue, waiting for notify...");
                    self.notify.notified().await;
                }
            }
        };
        let service = tokio::task::spawn(async move {
            select! {
                _ = fut => {},
                _ = cancelation => {},
            }
        });
        if let Ok(mut handles) = self.handles.lock() {
            handles.push(service);
        }
    }

    pub fn spawn_system(
        &'static self,
        cancelation: impl Future<Output = ()> + Clone + Send + 'static,
        sender: MessageHandler,
    ) {
        for _ in 0..self.parallel_downloads {
            self.run_service_stream(cancelation.clone(), sender.clone());
        }
    }

    pub fn clean(
        &'static self,
        cancelation: impl Future<Output = ()> + Clone + Send + 'static,
        sender: MessageHandler,
    ) {
        if let Ok(mut list) = self.download_list.lock() {
            list.clear();
        }
        if let Ok(mut set) = self.in_download.lock() {
            set.clear();
        }
        if let Ok(handles) = self.handles.lock() {
            for i in handles.iter() {
                i.abort()
            }
        }
        if let Ok(mut handle) = self.handles.lock() {
            handle.clear();
        }
        self.spawn_system(cancelation, sender);
    }

    pub fn set_download_list(&self, to_add: impl IntoIterator<Item = YoutubeMusicVideoRef>) {
        if let Ok(mut list) = self.download_list.lock() {
            list.clear();
            list.extend(to_add);
        }
        self.notify.notify_one();
    }

    pub fn add_to_download_list(&self, to_add: impl IntoIterator<Item = YoutubeMusicVideoRef>) {
        if let Ok(mut list) = self.download_list.lock() {
            let was_empty = list.is_empty();
            list.extend(to_add);
            if was_empty {
                self.notify.notify_one();
            }
        }
    }
}
