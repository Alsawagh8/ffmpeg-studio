<div dir="rtl">

# 🎬 FFmpeg Studio (واجهة رسومية أصيلة لـ FFmpeg على Wayland)

واجهة رسومية سريعة، خفيفة، وأصيلة لبيئة **Wayland** لأداة **FFmpeg**، مبنية بالكامل بلغة **Rust** ومكتبتي **GTK4** و **Libadwaita**. مصممة خصيصاً لنظام **Arch Linux** وتوزيعات لينكس الحديثة.

---

## ✨ المميزات الرئيسية

- **⚡ أداء فائق وسرعة استجابة:** مبنية بلغة Rust كملف تنفيذي أصيل باستهلاك ضئيل جداً للذاكرة وإقلاع فوري.
- **🖥️ دعم أصيل لـ Wayland:** تعمل مباشرة على Wayland دون الحاجة لطبقة التوافق XWayland.
- **🔍 قراءة ديناميكية شاملة:** تستخرج تلقائياً جميع ترميزات الفيديو والصوت، الصيغ والامتدادات، والفلاتر المثبتة فعلياً في `ffmpeg` على جهازك.
- **🎛️ تكيّف ذكي لخيارات الضغط (Smart Presets):** تغيّر قائمة سرعة الضغط تلقائياً حسب الترميز المختار (مثل الأرقام `0–13` لـ `libsvtav1`، و `p1–p7` لكروت `nvenc`، و `ultrafast–veryslow` لـ `x264/x265`).
- **🎨 قوائم جاهزة للفلاتر الشائعة:** اختصارات سريعة لتغيير ورفع الدقة (**Lanczos**)، مساحات الألوان (**BT.709** وتحويل **HDR إلى SDR**)، وفلاتر زيادة الحدة وإزالة التشويش.
- **ℹ️ توثيق مدمج لكل خيار:** أزرار مساعدة فورية تعرض كل الوسائط والمعاملات الخاصة بأي ترميز أو صيغة أو فلتر من `ffmpeg -h` داخل التطبيق.
- **📊 شريط تقدم حي ومعاينة مباشرة:** حساب دقيق لنسبة التقدم عبر `ffprobe`، دعم السحب والإفلات (Drag & Drop)، زر إيقاف فوري، وإمكانية تعديل أمر `ffmpeg` يدوياً قبل التشغيل.

---

## 🛠️ طريقة التثبيت على Arch Linux

تثبيت الحزم المطلوبة:
```bash
sudo pacman -S --needed rustup base-devel pkgconf gtk4 libadwaita ffmpeg
rustup default stable
```

تحميل المشروع وبناء النسخة النهائية السريعة:
```bash
git clone https://github.com
cd ffmpeg-studio
cargo build --release
```

تشغيل البرنامج:
```bash
./target/release/ffmpeg-gui
```

</div>

---

# 🎬 FFmpeg Studio (Native Wayland GUI)

A fast, lightweight, and native **Wayland** graphical user interface for **FFmpeg**, written in **Rust** using **GTK4** and **Libadwaita**. Designed primarily for **Arch Linux** and modern Linux desktops.

## ✨ Features

- **⚡ Native Performance:** Built in Rust with minimal RAM usage and instant startup.
- **🖥️ Native Wayland Support:** Powered by GTK4 & Libadwaita (no XWayland required).
- **🔍 Dynamic Capability Detection:** Automatically reads all installed video/audio encoders, muxers/formats, and filters directly from your system's `ffmpeg`.
- **🎛️ Smart Preset Adaptation:** Automatically switches preset options based on the selected encoder (`0–13` for `libsvtav1`, `p1–p7` for `nvenc`, `ultrafast–veryslow` for `x264/x265`).
- **🎨 Quick Filter Presets:** Built-in quick selectors for **Resolution/Upscaling (Lanczos)**, **Color Spaces (BT.709, HDR-to-SDR)**, and **Enhancement (Sharpen, Denoise)**.
- **ℹ️ Built-in Documentation:** Inspect exact flags and options for any codec, format, or filter (`ffmpeg -h`) directly inside the app.
- **📊 Real-Time Progress & Live Command:** Live progress bar via `ffprobe` + `-progress pipe:1`, drag-and-drop support, instant cancel button, and editable live command preview.

## 🛠️ Installation

### Arch Linux
```bash
sudo pacman -S --needed rustup base-devel pkgconf gtk4 libadwaita ffmpeg
rustup default stable
git clone https://github.com
cd ffmpeg-studio
cargo build --release
./target/release/ffmpeg-gui
```

### Ubuntu / Debian
```bash
sudo apt install cargo pkg-config libgtk-4-dev libadwaita-1-dev ffmpeg
git clone https://github.com
cd ffmpeg-studio
cargo build --release
```

### Fedora
```bash
sudo dnf install cargo pkgconf gtk4-devel libadwaita-devel ffmpeg
git clone https://github.com
cd ffmpeg-studio
cargo build --release
```

## 📄 License
MIT License
