# 🎬 FFmpeg Studio (Native Wayland GUI)

A fast, lightweight, and native **Wayland** graphical user interface for **FFmpeg**, written in **Rust** using **GTK4** and **Libadwaita**. Designed primarily for **Arch Linux** and modern Linux desktops.

---

## ✨ Features / المميزات

- **⚡ Native Performance:** Built in Rust with minimal RAM usage and instant startup.
- **🖥️ Native Wayland Support:** Powered by GTK4 & Libadwaita (no XWayland required).
- **🔍 Dynamic Capability Detection:** Automatically reads all installed video/audio encoders, muxers/formats, and filters directly from your system's `ffmpeg`.
- **🎛️ Smart Preset Adaptation:** Automatically switches preset options based on the selected encoder (e.g., `0–13` for `libsvtav1`, `p1–p7` for `nvenc`, `ultrafast–veryslow` for `x264/x265`).
- **🎨 Quick Filter Presets:** Built-in quick selectors for **Resolution/Upscaling (Lanczos)**, **Color Spaces (BT.709, HDR-to-SDR)**, and **Enhancement (Sharpen, Denoise)**.
- **ℹ️ Built-in Documentation:** Inspect exact flags and options for any codec, format, or filter (`ffmpeg -h`) directly inside the app.
- **📊 Real-Time Progress & Live Command:** Live progress bar via `ffprobe` + `-progress pipe:1`, drag-and-drop support, instant cancel button, and editable live command preview.

---

## 🛠️ Installation / التثبيت

### 1. Arch Linux
Install the required dependencies:
```bash
sudo pacman -S --needed rustup base-devel pkgconf gtk4 libadwaita ffmpeg
rustup default stable
```

Clone and build the release binary:
```bash
git clone https://github.com
cd ffmpeg-studio
cargo build --release
```

Run the application:
```bash
./target/release/ffmpeg-gui
```

### 2. Ubuntu / Debian
```bash
sudo apt install cargo pkg-config libgtk-4-dev libadwaita-1-dev ffmpeg
git clone https://github.com
cd ffmpeg-studio
cargo build --release
```

### 3. Fedora
```bash
sudo dnf install cargo pkgconf gtk4-devel libadwaita-devel ffmpeg
git clone https://github.com
cd ffmpeg-studio
cargo build --release
```

---

## 📄 License
MIT License
