use std::{fs::OpenOptions, io::{Read, Write}, path::Path, sync::{Arc, RwLock}};

use varuint::{ReadVarint, WriteVarint};
use ytpapi2::YoutubeMusicVideoRef;

use crate::YTLocalDatabase;

/// Synchronous write for internal use (async wrappers call this)
pub fn write_sync_with_db(cache_dir: &Path, references: &Arc<RwLock<Vec<YoutubeMusicVideoRef>>>) -> std::io::Result<()> {
    let db = match references.read() {
        Ok(db) => db,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(cache_dir.join("db.bin"))?;
    for video in &*db {
        write_video(&mut file, video)?;
    }
    Ok(())
}

/// Writes a video to a file
pub fn write_video(buffer: &mut impl Write, video: &YoutubeMusicVideoRef) -> std::io::Result<()> {
    write_str(buffer, &video.title)?;
    write_str(buffer, &video.author)?;
    write_str(buffer, &video.album)?;
    write_str(buffer, &video.video_id)?;
    write_str(buffer, &video.duration)?;
    Ok(())
}

/// Writes a string from the cursor
fn write_str(cursor: &mut impl Write, value: &str) -> std::io::Result<()> {
    cursor.write_varint(value.len() as u32)?;
    cursor.write_all(value.as_bytes())?;
    Ok(())
}

/// Synchronous write for internal use (sync wrapper)
pub fn write_sync(db: &YTLocalDatabase) {
    let references = db.references.clone();
    let _ = write_sync_with_db(&db.cache_dir, &references);
}

/// Fixes the database by reading from the binary file and rewriting it
pub fn fix_db_sync(db: &YTLocalDatabase) {
    let mut database = match db.references.write() {
        Ok(d) => d,
        Err(poisoned) => poisoned.into_inner(),
    };
    database.clear();
    fix_db_populate(&db.cache_dir, &mut database);
}

/// Populates database from binary file
pub fn fix_db_populate(cache_dir: &std::path::Path, database: &mut Vec<YoutubeMusicVideoRef>) {
    let _ = std::fs::read(cache_dir.join("db.bin")).map(|bytes| {
        let mut buffer = std::io::Cursor::new(bytes);
        while buffer.get_mut().len() > buffer.position() as usize {
            if let Some(video) = read_video(&mut buffer) {
                database.push(video);
            }
        }
    });
}

/// Reads a video from the cursor (for fix_db)
fn read_video(buffer: &mut std::io::Cursor<Vec<u8>>) -> Option<YoutubeMusicVideoRef> {
    Some(YoutubeMusicVideoRef {
        title: read_str(buffer)?,
        author: read_str(buffer)?,
        album: read_str(buffer)?,
        video_id: read_str(buffer)?,
        duration: read_str(buffer)?,
    })
}

/// Reads a string from the cursor
fn read_str(cursor: &mut std::io::Cursor<Vec<u8>>) -> Option<String> {
    let mut buf = vec![0u8; ReadVarint::<u32>::read_varint(cursor).ok()? as usize];
    cursor.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}