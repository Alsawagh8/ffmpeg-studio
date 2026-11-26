#!/bin/bash
set -e
echo "Building FFmpeg Studio (Release)..."
cargo build --release

mkdir -p ~/.local/bin ~/.local/share/icons/hicolor/scalable/apps ~/.local/share/applications
cp target/release/ffmpeg-gui ~/.local/bin/ffmpeg-gui
cp assets/ffmpeg-studio.svg ~/.local/share/icons/hicolor/scalable/apps/ffmpeg-studio.svg

cat << 'DESKTOP' > ~/.local/share/applications/ffmpeg-studio.desktop
[Desktop Entry]
Name=FFmpeg Studio
Comment=FFmpeg GUI for Wayland
Exec=ffmpeg-gui
Icon=ffmpeg-studio
Terminal=false
Type=Application
Categories=AudioVideo;Video;Audio;
DESKTOP

update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
echo "✅ تم تثبيت FFmpeg Studio وأيقونته بنجاح!"
