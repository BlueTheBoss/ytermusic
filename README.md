# YTerMusic

![YTeRMUSiC](./assets/banner/YTeRMUSiC.png "YTeRMUSiC")

YTerMusic is a TUI based Youtube Music Player that aims to be as fast and simple as possible.

## Screenshots
<p>
  <img
  src="./assets/screenshots/Choose-A-Playlist.png"
  alt="Choose a playlist"
  title="Choose a Playlist"
  />
  <img
  src="./assets/screenshots/Playlist-All.gif"
  alt="Playlist RGB"
  title="Playlist RGB"
  />
</p>


## Features

- Play your Youtube Music Playlist and Supermix.
- Memory efficient (Around 20MB of RAM while fully loaded)
- Cache all downloads and store them
- Work even without connection (If musics were already downloaded)
- Automatic background download manager
- **No browser cookies required** — OAuth device flow or anonymous mode
- **Synced lyrics** (fetched from LrcLib, auto-scrolls with playback)
- **Discord Rich Presence** (shows current song)
- **Last.fm / ListenBrainz scrobbling** (automatic with 50%/4min threshold)
- **Desktop notifications** on track change
- **Repeat modes** (repeat one, repeat all)
- **Queue view** (view, reorder, and remove upcoming songs)
- **Configurable keybindings** (customize all shortcuts in `config.toml`)
- Custom theming (You can use hex! #05313d = ![05313d](./assets/hex/05313d.png "#05313d") )

## Install

### Quick install (auto-detect distro)
```sh
git clone https://github.com/BlueTheBoss/ytermusic
cd ytermusic
./install.sh
```

### Using Make (any Linux)
```sh
git clone https://github.com/BlueTheBoss/ytermusic
cd ytermusic
make
sudo make install
```

### Arch Linux (manual)
```sh
sudo pacman -S --needed base-devel alsa-lib dbus pkg-config cargo
git clone https://github.com/BlueTheBoss/ytermusic
cd ytermusic
cargo build --release
sudo install -Dm755 target/release/ytermusic /usr/local/bin/ytermusic
```

### Debian/Ubuntu (manual)
```sh
sudo apt install build-essential libasound2-dev libdbus-1-dev pkg-config cargo
git clone https://github.com/BlueTheBoss/ytermusic
cd ytermusic
cargo build --release
sudo install -Dm755 target/release/ytermusic /usr/local/bin/ytermusic
```

### From source (any platform)
```sh
git clone https://github.com/BlueTheBoss/ytermusic
cd ytermusic
cargo build --release
# Binary at target/release/ytermusic
```

## Setup

No browser or cookie setup needed. On first launch you can:
- **Use anonymously** — works immediately, no account required
- **Log in with OAuth** — press `L` in the playlist or player screen, follow the device code URL

### Configuration

Create `~/.config/ytermusic/config.toml` to customize:

```toml
[discord]
client_id = "your_discord_app_id"

[scrobble]
lastfm_api_key = "..."
lastfm_session = "..."
listenbrainz_token = "..."

[keybinds]
play_pause = " "
search = "f"
shuffle = "s"
remove = "r"
queue_view = "Q"
lyrics = "y"
login = "L"
repeat_mode = "m"
forward = "l"
backward = "h"
volume_up = "+"
volume_down = "-"
scroll_up = "k"
scroll_down = "j"
enter = "enter"
escape = "esc"
cleanup = "C"
next = ">"
previous = "<"
```

All keybindings have sensible defaults — only include the ones you want to override.

## Usage

- Use your mouse to <kbd>click</kbd> in lists if your terminal has mouse support
- Press <kbd>Space</kbd> to play/pause
- Press <kbd>Enter</kbd> to select a playlist or a music
- Press <kbd>f</kbd> to search
- Press <kbd>s</kbd> to shuffle
- Press <kbd>r</kbd> to remove a music from the main playlist
- Press <kbd>y</kbd> to open synced lyrics view
- Press <kbd>Q</kbd> to view and reorder the upcoming queue
- Press <kbd>m</kbd> to cycle repeat modes (none → repeat one → repeat all)
- Press <kbd>L</kbd> to log in with OAuth
- Press <kbd>Arrow Right</kbd> or <kbd>&gt;</kbd> to skip 5 seconds
- Press <kbd>Arrow Left</kbd> or <kbd>&lt;</kbd> to go back 5 seconds
- Press <kbd>CTRL</kbd> + <kbd>Arrow Right</kbd> to go to the next song
- Press <kbd>CTRL</kbd> + <kbd>Arrow Left</kbd> to go to the previous song
- Press <kbd>+</kbd> for volume up
- Press <kbd>-</kbd> for volume down
- Press <kbd>Arrow down</kbd> to scroll down
- Press <kbd>Arrow up</kbd> to scroll up
- Press <kbd>ESC</kbd> to exit the current menu
- Press <kbd>CTRL</kbd> + <kbd>C</kbd> or <kbd>CTRL</kbd> + <kbd>D</kbd> to exit

## How to fix common issues

If you have any issues start by running:
```sh
ytermusic --fix-db
```
This will try to fix any issues with the cache database.

If you still have issues, you can clear the cache by running:
```sh
ytermusic --clear-cache
```
