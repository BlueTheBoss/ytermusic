use std::path::PathBuf;

use log::warn;
use once_cell::sync::Lazy;

use crate::{config, utils::get_project_dirs};

pub static CACHE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let pdir = get_project_dirs();
    if let Some(dir) = pdir {
        return dir.cache_dir().to_path_buf();
    };
    warn!("Failed to get cache dir! Defaulting to './data'");
    PathBuf::from("./data")
});

pub static CONFIG: Lazy<config::Config> = Lazy::new(config::Config::new);

pub const INTRODUCTION: &str = r#"Usage: ytermusic [options]

YTerMusic is a TUI based Youtube Music Player that aims to be as fast and simple as possible.
No browser or configuration file needed — just run and enjoy!

Login from within the app (press 'L') to access your library and playlists.

Options:
        -h or --help        Show this menu
        --fix-db            Fix the database in cache
        --clear-cache       Erase all the files in cache

Shortcuts:
        Use your mouse to click in lists if your terminal has mouse support
        Space                     play/pause
        Enter                     select a playlist or a music
        f                         search
        L                         login to YouTube Music
        s                         shuffle
        r                         remove a music from the main playlist
        Arrow Right or >          skip 5 seconds
        Arrow Left or <           go back 5 seconds
        CTRL + Arrow Right (>)    go to the next song
        CTRL + Arrow Left  (<)    go to the previous song
        +                         volume up
        -                         volume down
        Arrow down                scroll down
        Arrow up                  scroll up
        ESC                       exit the current menu
        CTRL + C or CTRL + D      quit
"#;
