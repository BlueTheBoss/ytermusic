#!/usr/bin/env bash
set -euo pipefail

echo "==> YTerMusic Installer"
echo ""

# Detect distro for deps
if command -v pacman &>/dev/null; then
    echo "[*] Detected Arch Linux"
    sudo pacman -S --needed --noconfirm base-devel alsa-lib dbus pkg-config cargo
elif command -v apt &>/dev/null; then
    echo "[*] Detected Debian/Ubuntu"
    sudo apt install -y build-essential libasound2-dev libdbus-1-dev pkg-config cargo
elif command -v dnf &>/dev/null; then
    echo "[*] Detected Fedora"
    sudo dnf install -y gcc alsa-lib-devel dbus-devel pkg-config cargo
elif command -v zypper &>/dev/null; then
    echo "[*] Detected openSUSE"
    sudo zypper install -y gcc alsa-devel dbus-1-devel pkg-config cargo
else
    echo "[!] Unsupported distro. Ensure you have: Rust, ALSA dev, dbus dev, pkg-config"
fi

echo ""
echo "[*] Building ytermusic..."
cargo build --release

echo ""
echo "[*] Installing to /usr/local/bin..."
sudo install -Dm755 target/release/ytermusic /usr/local/bin/ytermusic

echo ""
echo "==> Done! Run 'ytermusic' to start."
