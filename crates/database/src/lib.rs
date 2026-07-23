use std::{fs::OpenOptions, path::PathBuf, sync::RwLock};

mod reader;
mod writer;

pub use writer::write_video;
use ytpapi2::YoutubeMusicVideoRef;

pub struct YTLocalDatabase {
    cache_dir: PathBuf,
    references: RwLock<Vec<YoutubeMusicVideoRef>>,
}

impl YTLocalDatabase {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            references: RwLock::new(Vec::new()),
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
        write_video(&mut file, &video);
        let mut db = match self.references.write() {
            Ok(db) => db,
            Err(poisoned) => poisoned.into_inner(),
        };
        db.push(video);
    }
}
