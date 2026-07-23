use std::{fs::OpenOptions, path::PathBuf, sync::{Arc, RwLock}};

mod reader;
mod writer;

pub use writer::write_video;
use tokio::task::spawn_blocking;
use ytpapi2::YoutubeMusicVideoRef;

pub struct YTLocalDatabase {
    cache_dir: PathBuf,
    references: Arc<RwLock<Vec<YoutubeMusicVideoRef>>>,
}

impl YTLocalDatabase {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            references: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn clone_from(&self, videos: &Vec<YoutubeMusicVideoRef>) {
        let mut db = match self.references.write() {
            Ok(db) => db,
            Err(poisoned) => poisoned.into_inner(),
        };
        db.clone_from(videos);
    }

    pub fn remove_video(&self, video: &YoutubeMusicVideoRef) {
        let mut database = match self.references.write() {
            Ok(db) => db,
            Err(poisoned) => poisoned.into_inner(),
        };
        database.retain(|v| v.video_id != video.video_id);
        drop(database);
        self.write();
    }

    pub async fn remove_video_async(&self, video: &YoutubeMusicVideoRef) {
        let video_id = video.video_id.clone();
        {
            let mut database = match self.references.write() {
                Ok(db) => db,
                Err(poisoned) => poisoned.into_inner(),
            };
            database.retain(|v| v.video_id != video_id);
        }
        self.write_async().await;
    }

    pub fn append(&self, video: YoutubeMusicVideoRef) {
        let mut file = match OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.cache_dir.join("db.bin"))
        {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to open database for append: {e}");
                return;
            }
        };
        write_video(&mut file, &video).ok();
        let mut db = match self.references.write() {
            Ok(db) => db,
            Err(poisoned) => poisoned.into_inner(),
        };
        db.push(video);
    }

    pub async fn append_async(&self, video: YoutubeMusicVideoRef) {
        let cache_dir = self.cache_dir.clone();
        let video_clone = video.clone();
        spawn_blocking(move || {
            let mut file = match OpenOptions::new()
                .append(true)
                .create(true)
                .open(cache_dir.join("db.bin"))
            {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to open database for append: {e}");
                    return;
                }
            };
            write_video(&mut file, &video_clone).ok();
        }).await.ok();

        let mut db = match self.references.write() {
            Ok(db) => db,
            Err(poisoned) => poisoned.into_inner(),
        };
        db.push(video);
    }

    pub fn read(&self) -> Option<Vec<YoutubeMusicVideoRef>> {
        reader::read_sync(&self.cache_dir)
    }

    pub async fn read_async(&self) -> Option<Vec<YoutubeMusicVideoRef>> {
        let cache_dir = self.cache_dir.clone();
        spawn_blocking(move || reader::read_sync(&cache_dir)).await.ok().flatten()
    }

    pub fn write(&self) {
        writer::write_sync(self);
    }

    pub async fn write_async(&self) {
        let cache_dir = self.cache_dir.clone();
        let references = self.references.clone();
        spawn_blocking(move || {
            let _ = writer::write_sync_with_db(&cache_dir, &references);
        }).await.ok();
    }

    pub fn fix_db(&self) {
        writer::fix_db_sync(self);
    }

    pub async fn fix_db_async(&self) {
        let cache_dir = self.cache_dir.clone();
        let references = self.references.clone();
        spawn_blocking(move || {
            let mut db = match references.write() {
                Ok(db) => db,
                Err(poisoned) => poisoned.into_inner(),
            };
            db.clear();
            writer::fix_db_populate(&cache_dir, &mut db);
        }).await.ok();
    }
}
