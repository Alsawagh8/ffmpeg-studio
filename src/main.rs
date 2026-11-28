#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::path::PathBuf;
use adw::prelude::*;
use gtk::{
    gdk, gio, glib, Box, Button, DropDown, Entry, FileDialog, Label, Orientation, ProgressBar,
    PropertyExpression, ScrolledWindow, StringFilterMatchMode, StringList, StringObject,
    TextView,
};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

const APP_ID: &str = "org.archlinux.FfmpegGui";

fn is_arabic_system() -> bool {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(val) = std::env::var(var) {
            if val.trim().to_lowercase().starts_with("ar") { return true; }
        }
    }
    for lang in glib::language_names() {
        let l = lang.to_lowercase();
        if l.starts_with("ar") { return true; }
        if l != "c" && l != "posix" && !l.is_empty() { return false; }
    }
    false
}


struct FfmpegCapabilities {
    video_encoders: Vec<String>,
    audio_encoders: Vec<String>,
    formats: Vec<String>,
    video_filters: Vec<String>,
    audio_filters: Vec<String>,
}

impl FfmpegCapabilities {
    fn detect() -> Self {
        let mut video_encoders = vec!["copy".to_string(), (if is_arabic_system() { "none (بدون فيديو)" } else { "none (No Video)" }).to_string()];
        let mut audio_encoders = vec!["copy".to_string(), (if is_arabic_system() { "none (بدون صوت)" } else { "none (No Audio)" }).to_string()];
        let mut formats: Vec<String> = ["mp4","mkv","webm","mov","avi","mp3","m4a","flac","wav","opus","ogg"].iter().map(|x| x.to_string()).collect();
        let mut video_filters = Vec::new();
        let mut audio_filters = Vec::new();

        if let Ok(output) = new_std_cmd("ffmpeg")
            .args(["-hide_banner", "-encoders"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('V')
                    && trimmed.len() > 8
                    && !trimmed.starts_with("V..... =")
                {
                    if let Some(name) = trimmed[7..].split_whitespace().next() {
                        video_encoders.push(name.to_string());
                    }
                } else if trimmed.starts_with('A')
                    && trimmed.len() > 8
                    && !trimmed.starts_with("A..... =")
                {
                    if let Some(name) = trimmed[7..].split_whitespace().next() {
                        audio_encoders.push(name.to_string());
                    }
                }
            }
        }

        if let Ok(output) = new_std_cmd("ffmpeg")
            .args(["-hide_banner", "-muxers"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("E ") && trimmed.len() > 4 {
                    if let Some(name) = trimmed[3..].split_whitespace().next() {
                        let raw = name.split(',').next().unwrap_or(name); let primary_ext = if raw == "matroska" { "mkv" } else { raw }; if ["null","hash","framehash","crc","framecrc","uncodedframecrc","ffmetadata","mkvtimestamp_v2","streamhash"].contains(&primary_ext) { continue; }
                        if !formats.contains(&primary_ext.to_string()) {
                            formats.push(primary_ext.to_string());
                        }
                    }
                }
            }
        }

        // استخراج الفلاتر بدقة عبر البحث عن رمز الاتجاه -> في العمود الثالث
        if let Ok(output) = new_std_cmd("ffmpeg")
            .args(["-hide_banner", "-filters"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 3 && parts[2].contains("->") {
                    let filter_name = parts[1].to_string();
                    let io_dir = parts[2];
                    if io_dir.starts_with('V') || io_dir.ends_with('V') {
                        video_filters.push(filter_name);
                    } else if io_dir.starts_with('A') || io_dir.ends_with('A') {
                        audio_filters.push(filter_name);
                    }
                }
            }
        }

        Self {
            video_encoders,
            audio_encoders,
            formats,
            video_filters,
            audio_filters,
        }
    }
}

enum PresetMode {
    None,
    StandardPreset,
    SvtAv1,
    Nvenc,
    CpuUsed,
}

fn detect_preset_mode(codec: &str) -> PresetMode {
    let c = codec.to_lowercase();
    if c == "copy" || c.starts_with("none") || c.contains("vaapi") || c.contains("qsv") {
        PresetMode::None
    } else if c.contains("svtav1") {
        PresetMode::SvtAv1
    } else if c.contains("nvenc") {
        PresetMode::Nvenc
    } else if c.contains("vpx") || c.contains("aom") {
        PresetMode::CpuUsed
    } else {
        PresetMode::StandardPreset
    }
}

fn preset_options_for_mode(mode: &PresetMode) -> Vec<&'static str> {
    match mode {
        PresetMode::None => vec![(if is_arabic_system() { "غير متاح لهذا الترميز" } else { "Not available for this codec" })],
        PresetMode::StandardPreset => vec![
            (if is_arabic_system() { "افتراضي" } else { "Default" }),
            "ultrafast",
            "superfast",
            "veryfast",
            "faster",
            "fast",
            "medium",
            "slow",
            "slower",
            "veryslow",
        ],
        PresetMode::SvtAv1 => vec![
            (if is_arabic_system() { "افتراضي" } else { "Default" }),
            (if is_arabic_system() { "0 (أبطأ / أعلى جودة)" } else { "0 (Slowest / Best Quality)" }),
            "2",
            (if is_arabic_system() { "4 (جودة عالية)" } else { "4 (High Quality)" }),
            "5",
            (if is_arabic_system() { "6 (متوازن)" } else { "6 (Balanced)" }),
            "7",
            (if is_arabic_system() { "8 (سريع)" } else { "8 (Fast)" }),
            (if is_arabic_system() { "10 (سريع جداً)" } else { "10 (Very Fast)" }),
            "12",
            (if is_arabic_system() { "13 (الأسرع)" } else { "13 (Fastest)" }),
        ],
        PresetMode::Nvenc => vec![
            (if is_arabic_system() { "افتراضي" } else { "Default" }),
            (if is_arabic_system() { "p1 (الأسرع)" } else { "p1 (Fastest)" }),
            "p2",
            "p3",
            (if is_arabic_system() { "p4 (متوازن)" } else { "p4 (Balanced)" }),
            "p5",
            "p6",
            (if is_arabic_system() { "p7 (أعلى جودة)" } else { "p7 (Best Quality)" }),
        ],
        PresetMode::CpuUsed => vec![
            (if is_arabic_system() { "افتراضي" } else { "Default" }),
            (if is_arabic_system() { "0 (أعلى جودة)" } else { "0 (Best Quality)" }),
            "1",
            "2",
            "3",
            (if is_arabic_system() { "4 (متوازن)" } else { "4 (Balanced)" }),
            "5",
            (if is_arabic_system() { "6 (سريع)" } else { "6 (Fast)" }),
            (if is_arabic_system() { "8 (الأسرع)" } else { "8 (Fastest)" }),
        ],
    }
}

fn parse_shell_args(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';

    for c in cmd.chars() {
        if in_quotes {
            if c == quote_char {
                in_quotes = false;
            } else {
                current.push(c);
            }
        } else if c == '"' || c == '\'' {
            in_quotes = true;
            quote_char = c;
        } else if c.is_whitespace() {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn format_cmd_preview(args: &[String]) -> String {
    let formatted: Vec<String> = args
        .iter()
        .map(|a| {
            if a.contains(' ')
                || a.contains('[')
                || a.contains(']')
                || a.contains('(')
                || a.contains(')')
            {
                format!("\"{}\"", a)
            } else {
                a.clone()
            }
        })
        .collect();
    format!("ffmpeg {}", formatted.join(" "))
}

fn escape_sub(p: &str) -> String { p.replace('\\', "/").replace(':', "\\:").replace('\'', "\\\'") }

fn new_std_cmd<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
    let mut cmd = std::process::new_std_cmd(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd
}

fn open_dir(p: &str) { let d = Path::new(p).parent().filter(|x| !x.as_os_str().is_empty()).unwrap_or(Path::new(".")); #[cfg(target_os = "windows")] let _ = new_std_cmd("explorer").arg(d).spawn(); #[cfg(target_os = "linux")] let _ = new_std_cmd("xdg-open").arg(d).spawn(); }

fn extract_video_thumbnails(video_path: &str) -> Vec<PathBuf> {
    let mut out_imgs = Vec::new();
    let dur_out = new_std_cmd("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1", video_path])
        .output();
    let dur: f64 = dur_out.ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<f64>().ok())
        .unwrap_or(10.0)
        .max(1.0);
    let tmp_dir = std::env::temp_dir().join("ffmpeg_studio_thumbs");
    let _ = std::fs::create_dir_all(&tmp_dir);
    for (idx, pct) in [0.15, 0.35, 0.60, 0.85].iter().enumerate() {
        let ts = format!("{:.2}", dur * pct);
        let img_path = tmp_dir.join(format!("thumb_{}.jpg", idx));
        let _ = std::fs::remove_file(&img_path);
        let st = new_std_cmd("ffmpeg")
            .args(["-y", "-ss", &ts, "-i", video_path, "-vframes", "1", "-vf", "scale=480:-2", "-q:v", "4", img_path.to_str().unwrap_or("")])
            .status();
        if st.map(|x| x.success()).unwrap_or(false) && img_path.exists() {
            out_imgs.push(img_path);
        }
    }
    out_imgs
}

fn probe_sub_tracks(path: &str, is_ar: bool) -> Vec<String> {
    let mut items = if is_ar { vec!["افتراضي (FFmpeg القياسي)".into(), "كل التراكات (نسخ جميع الترجمات)".into(), "مسح الكل (-sn)".into()] } else { vec!["Default (Standard FFmpeg)".into(), "All Tracks (Copy all)".into(), "Remove All (-sn)".into()] };
    if let Ok(out) = new_std_cmd("ffprobe").args(["-v","error","-select_streams","s","-show_entries","stream=codec_name:stream_tags=language,title","-of","csv=p=0:s=|",path]).output() {
        for (i, l) in String::from_utf8_lossy(&out.stdout).lines().enumerate() { let p: Vec<&str> = l.split('|').collect(); items.push(format!("{} {}: {} {} [{}]", if is_ar {"تراك"} else {"Track"}, i, p.get(1).unwrap_or(&""), p.get(2).unwrap_or(&""), p.first().unwrap_or(&"sub"))); }
    }
    items
}
fn build_default_output_path(input: &str, ext: &str) -> String {
    let p = Path::new(input);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            return format!("{}/{}(1).{}", parent.display(), stem, ext);
        }
    }
    format!("{}(1).{}", stem, ext)
}

fn get_ffmpeg_help(topic: &str, name: &str) -> String {
    if name.is_empty() || name == "copy" || name.starts_with("none") {
        return (if is_arabic_system() { "لا توجد خيارات متقدمة إضافية لهذا الاختيار." } else { "No additional advanced options for this selection." }).to_string();
    }
    let query = format!("{}={}", topic, name);
    if let Ok(output) = new_std_cmd("ffmpeg")
        .args(["-hide_banner", "-h", &query])
        .output()
    {
        let out = String::from_utf8_lossy(&output.stdout).to_string();
        if out.trim().is_empty() {
            return String::from_utf8_lossy(&output.stderr).to_string();
        }
        return out;
    }
    (if is_arabic_system() { "تعذر جلب معلومات المساعدة من ffmpeg" } else { "Failed to retrieve help info from ffmpeg" }).to_string()
}

fn get_media_duration(path: &str) -> f64 {
    let args = [
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
        path,
    ];
    if let Ok(output) = new_std_cmd("ffprobe").args(args).output() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return s.parse::<f64>().unwrap_or(0.0);
    }
    0.0
}

fn get_media_info(path: &str) -> String {
    let entries = "format=format_name,duration,size,bit_rate:stream=codec_name,codec_type,width,height,r_frame_rate,sample_rate";
    let args = [
        "-v",
        "error",
        "-show_entries",
        entries,
        "-of",
        "default=noprint_wrappers=1",
        path,
    ];
    if let Ok(output) = new_std_cmd("ffprobe").args(args).output() {
        return String::from_utf8_lossy(&output.stdout).to_string();
    }
    (if is_arabic_system() { "تعذر قراءة معلومات الملف" } else { "Unable to read media info" }).to_string()
}

enum ProgressMsg {
    Progress(f64, String),
    Done(Result<String, String>),
}

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn make_dropdown(items: &[String], searchable: bool) -> DropDown {
    let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
    let list = StringList::new(&refs);
    let expr = PropertyExpression::new(
        StringObject::static_type(),
        None::<gtk::Expression>,
        "string",
    );
    let dd = DropDown::new(Some(list), Some(expr));
    dd.set_enable_search(searchable);
    if searchable {
        dd.set_search_match_mode(StringFilterMatchMode::Substring);
    }
    dd
}

fn get_dropdown_text(dd: &DropDown, default: &str) -> String {
    dd.selected_item()
        .and_downcast::<StringObject>()
        .map(|s| s.string().to_string())
        .unwrap_or_else(|| default.to_string())
}


fn wrap_in_scroll(child: &impl IsA<gtk::Widget>) -> ScrolledWindow {
    ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(child)
        .build()
}

fn make_tab_box() -> Box {
    Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build()
}

fn show_help_dialog(parent: &adw::ApplicationWindow, title: &str, content_text: &str) {
    let dialog = adw::Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(680)
        .default_height(500)
        .build();

    let header = adw::HeaderBar::new();
    let text_view = TextView::builder()
        .editable(false)
        .monospace(true)
        .left_margin(12)
        .right_margin(12)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    text_view.buffer().set_text(content_text);

    let scroll = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&text_view)
        .build();

    let vbox = Box::builder().orientation(Orientation::Vertical).build();
    vbox.append(&header);
    vbox.append(&scroll);

    dialog.set_content(Some(&vbox));
    dialog.present();
}


fn build_quick_presets_box(vf_entry: &Entry) -> Box {
    let container = Box::builder().orientation(Orientation::Vertical).spacing(8).margin_bottom(8).build();
    let res_items = [
        (if is_arabic_system() { "اختر دقة جاهزة..." } else { "Select a resolution preset..." }),
        "4K UHD (3840x2160) - Lanczos -> scale=3840:2160:flags=lanczos",
        "2K QHD (2560x1440) - Lanczos -> scale=2560:1440:flags=lanczos",
        "1080p FullHD (1920x1080) -> scale=1920:1080:flags=lanczos",
        "720p HD (1280x720) -> scale=1280:720:flags=lanczos",
        "تناسب تلقائي للعرض 1920 -> scale=1920:-2:flags=lanczos",
    ];
    let color_items = [
        (if is_arabic_system() { "اختر مساحة ألوان أو تحسين لوني..." } else { "Select a color space or grading preset..." }),
        "مساحة ألوان قياسية HD (BT.709) -> colorspace=all=bt709:iall=bt601-6-625:fast=1",
        "تحويل HDR إلى SDR طبيعي -> zscale=t=linear:npl=100,format=gbrpf32le,tonemap=hable,zscale=p=bt709:t=bt709:m=bt709:r=tv,format=yuv420p",
        "نطاق لوني كامل (Full Range) -> scale=in_range=limited:out_range=full",
        "زيادة تشبع الألوان والتباين -> eq=contrast=1.08:saturation=1.20:brightness=0.02",
        "تعميق الأسود (Cinema Look) -> eq=gamma=0.95:contrast=1.12:saturation=1.15",
    ];
    let enhance_items = [
        (if is_arabic_system() { "اختر فلتر تحسين ووضوح..." } else { "Select an enhancement or sharpen filter..." }),
        "زيادة الحدة والوضوح (Sharpen) -> unsharp=5:5:1.0:5:5:0.0",
        "حدة فائقة للأنمي والرسوم -> unsharp=5:5:1.5:5:5:0.0",
        "إزالة التحبب والتشويش (Denoise خفيف) -> hqdn3d=1.5:1.5:6:6",
        "إزالة التشويش القوي (Denoise قوي) -> hqdn3d=4.0:3.0:6:4.5",
    ];
    for (title, items) in [
        ((if is_arabic_system() { "📐 تغيير ورفع الدقة (Resolution):" } else { "📐 Resolution & Upscaling:" }), res_items.as_slice()),
        ((if is_arabic_system() { "🎨 مساحات الألوان (Color Space):" } else { "🎨 Color Space & Grading:" }), color_items.as_slice()),
        ((if is_arabic_system() { "✨ تحسين الجودة والوضوح (Enhance):" } else { "✨ Enhancement & Denoise:" }), enhance_items.as_slice()),
    ] {
        let row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
        let dd = DropDown::new(Some(StringList::new(items)), gtk::Expression::NONE);
        dd.set_hexpand(true);
        let add_btn = Button::with_label((if is_arabic_system() { "➕ تطبيق" } else { "➕ Apply" }));
        let (dd_c, vf_c) = (dd.clone(), vf_entry.clone());
        add_btn.connect_clicked(move |_| {
            if let Some(obj) = dd_c.selected_item().and_downcast::<StringObject>() {
                if let Some((_, cmd)) = obj.string().to_string().split_once(" -> ") {
                    let cur = vf_c.text().trim().to_string();
                    vf_c.set_text(&if cur.is_empty() { cmd.to_string() } else { format!("{},{}", cur, cmd) });
                }
            }
        });
        row.append(&dd);
        row.append(&add_btn);
        container.append(&Label::builder().label(title).xalign(0.0).build());
        container.append(&row);
    }
    container
}

fn build_ui(app: &adw::Application) {
    let is_ar = is_arabic_system();
    gtk::Widget::set_default_direction(if is_ar { gtk::TextDirection::Rtl } else { gtk::TextDirection::Ltr });
    let caps = FfmpegCapabilities::detect();
    let selected_file: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let running_child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    let view_stack = adw::ViewStack::new();
    let view_switcher = adw::ViewSwitcher::builder()
        .stack(&view_stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&view_switcher));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("FFmpeg Studio v0.4.2")
        .default_width(780)
        .default_height(600)
        .width_request(360)
        .height_request(420)
        .build();

    // --- التبويب 1: الملف والصيغة ---
    let general_box = make_tab_box();
    let select_btn = Button::with_label((if is_arabic_system() { "📂 اختر ملف المصدر (أو اسحب الملف وأفلته هنا)" } else { "📂 Select Source File (or Drag & Drop here)" }));
    let file_label = Label::new(Some((if is_arabic_system() { "لم يتم اختيار ملف" } else { "No file selected" })));
    file_label.set_wrap(true);

    let format_row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let format_dropdown = make_dropdown(&caps.formats, true);
    format_dropdown.set_hexpand(true);
    let format_help_btn = Button::with_label((if is_arabic_system() { "ℹ️ خيارات الصيغة" } else { "ℹ️ Format Options" }));
    format_row.append(&format_dropdown);
    format_row.append(&format_help_btn);

    let output_name_entry = Entry::builder()
        .placeholder_text((if is_arabic_system() { "اسم الملف الناتج (اتركه فارغاً ليصبح الاسم(1).ext تلقائياً)" } else { "Output file path (leave empty for auto Name(1).ext)" }))
        .build();

    let media_info_label = Label::new(Some((if is_arabic_system() { "معلومات الملف ستظهر هنا..." } else { "Media information will appear here..." })));
    media_info_label.set_wrap(true);
    media_info_label.set_xalign(0.0);
    media_info_label.set_xalign(0.0);
    media_info_label.set_yalign(0.0);
    media_info_label.set_selectable(true);
    media_info_label.set_margin_top(12);
    media_info_label.set_margin_bottom(12);
    media_info_label.set_margin_start(14);
    media_info_label.set_margin_end(14);
    let info_scroll = ScrolledWindow::builder()
        .min_content_height(210)
        .vexpand(true)
        .hexpand(true)
        .css_classes(["card"])
        .child(&media_info_label)
        .build();

    general_box.append(&select_btn);
    general_box.append(&file_label);
    general_box.append(&Label::new(Some((if is_arabic_system() { "صيغة الإخراج (Format):" } else { "Output Format:" }))));
    general_box.append(&format_row);
    general_box.append(&Label::new(Some((if is_arabic_system() { "مسار الحفظ المخصص:" } else { "Custom Output Path:" }))));
    general_box.append(&output_name_entry);
    general_box.append(&Label::new(Some((if is_arabic_system() { "تفاصيل الملف (ffprobe):" } else { "Media Details (ffprobe):" }))));
    let split_area = Box::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .vexpand(true)
        .hexpand(true)
        .homogeneous(false)
        .build();
    info_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    info_scroll.set_size_request(360, -1);
    media_info_label.set_wrap(true);
    media_info_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);

    let preview_slot = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .vexpand(true)
        .hexpand(true)
        .css_classes(["card"])
        .build();
    let prev_hint = Label::new(Some(if is_ar { "🖼️ منطقة معاينة لقطات الفيديو" } else { "🖼️ Video Preview Frames Area" }));
    prev_hint.set_vexpand(true);
    prev_hint.set_opacity(0.45);
    preview_slot.append(&prev_hint);

        split_area.append(&info_scroll);
    split_area.append(&preview_slot);
    general_box.append(&split_area);
    split_area.append(&info_scroll);
    split_area.append(&preview_slot);
    general_box.append(&split_area);
    view_stack.add_titled(&wrap_in_scroll(&general_box), Some("general"), if is_ar { "الملف والصيغة" } else { "File & Format" });

    // --- التبويب 2: الفيديو ---
    let video_box = make_tab_box();
    let video_row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let video_dropdown = make_dropdown(&caps.video_encoders, true);
    video_dropdown.set_hexpand(true);
    let video_help_btn = Button::with_label((if is_arabic_system() { "ℹ️ خيارات الترميز" } else { "ℹ️ Codec Options" }));
    video_row.append(&video_dropdown);
    video_row.append(&video_help_btn);

    let crf_entry = Entry::builder().placeholder_text((if is_arabic_system() { "مثال: 23 (أو 30 لـ svtav1)" } else { "e.g. 23 (or 30 for svtav1)" })).build();
    let v_bitrate_entry = Entry::builder().placeholder_text((if is_arabic_system() { "مثال: 5000k" } else { "e.g. 5000k" })).build();
    let fps_entry = Entry::builder().placeholder_text((if is_arabic_system() { "مثال: 30 أو 60" } else { "e.g. 30 or 60" })).build();

    let preset_label = Label::new(Some((if is_arabic_system() { "سرعة الضغط (-preset):" } else { "Encoding Speed (-preset):" })));
    let preset_dropdown = DropDown::new(
        Some(StringList::new(&[(if is_arabic_system() { "غير متاح لهذا الترميز" } else { "Not available for this codec" })])),
        gtk::Expression::NONE,
    );
    preset_dropdown.set_sensitive(false);

    let extra_v_args = Entry::builder()
        .placeholder_text((if is_arabic_system() { "وسائط فيديو إضافية (مثال: -pix_fmt yuv420p)" } else { "Extra video flags (e.g. -pix_fmt yuv420p)" }))
        .build();

    video_box.append(&Label::new(Some((if is_arabic_system() { "ترميز الفيديو (-c:v):" } else { "Video Codec (-c:v):" }))));
    video_box.append(&video_row);
    video_box.append(&Label::new(Some((if is_arabic_system() { "معامل الجودة (-crf):" } else { "Quality Factor (-crf):" }))));
    video_box.append(&crf_entry);
    video_box.append(&Label::new(Some((if is_arabic_system() { "معدل البت (-b:v):" } else { "Video Bitrate (-b:v):" }))));
    video_box.append(&v_bitrate_entry);
    video_box.append(&Label::new(Some((if is_arabic_system() { "معدل الإطارات (-r):" } else { "Frame Rate (-r):" }))));
    video_box.append(&fps_entry);
    video_box.append(&preset_label);
    video_box.append(&preset_dropdown);
    let pix_dd = DropDown::new(Some(StringList::new(&if is_ar { vec!["عمق الألوان: 8-bit قياسي (افتراضي)","10-bit (yuv420p10le)","12-bit (yuv420p12le)","تلقائي حسب المصدر"] } else { vec!["Color Depth: 8-bit (Default)","10-bit (yuv420p10le)","12-bit (yuv420p12le)","Auto (Source)"] })), gtk::Expression::NONE);
    video_box.append(&pix_dd);
    video_box.append(&Label::new(Some((if is_arabic_system() { "خيارات فيديو مخصصة:" } else { "Custom Video Flags:" }))));
    video_box.append(&extra_v_args);
    view_stack.add_titled(&wrap_in_scroll(&video_box), Some("video"), (if is_arabic_system() { "الفيديو" } else { "Video" }));

    // --- التبويب 3: الصوت ---
    let audio_box = make_tab_box();
    let audio_row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let audio_dropdown = make_dropdown(&caps.audio_encoders, true);
    audio_dropdown.set_hexpand(true);
    let audio_help_btn = Button::with_label((if is_arabic_system() { "ℹ️ خيارات الترميز" } else { "ℹ️ Codec Options" }));
    audio_row.append(&audio_dropdown);
    audio_row.append(&audio_help_btn);

    let a_bitrate_entry = Entry::builder().placeholder_text((if is_arabic_system() { "مثال: 192k" } else { "e.g. 192k" })).build();
    let sample_rate_entry = Entry::builder().placeholder_text((if is_arabic_system() { "مثال: 48000" } else { "e.g. 48000" })).build();
    let channels_entry = Entry::builder().placeholder_text((if is_arabic_system() { "مثال: 2" } else { "e.g. 2" })).build();

    audio_box.append(&Label::new(Some((if is_arabic_system() { "ترميز الصوت (-c:a):" } else { "Audio Codec (-c:a):" }))));
    audio_box.append(&audio_row);
    audio_box.append(&Label::new(Some((if is_arabic_system() { "معدل البت (-b:a):" } else { "Audio Bitrate (-b:a):" }))));
    audio_box.append(&a_bitrate_entry);
    audio_box.append(&Label::new(Some((if is_arabic_system() { "تردد العينات (-ar):" } else { "Sample Rate (-ar):" }))));
    audio_box.append(&sample_rate_entry);
    audio_box.append(&Label::new(Some((if is_arabic_system() { "عدد القنوات (-ac):" } else { "Audio Channels (-ac):" }))));
    audio_box.append(&channels_entry);
    view_stack.add_titled(&wrap_in_scroll(&audio_box), Some("audio"), (if is_arabic_system() { "الصوت" } else { "Audio" }));

    let sub_box = make_tab_box();
    let hb_bar = Box::builder().orientation(Orientation::Horizontal).spacing(6).build();
    let hb_all_btn = Button::with_label(if is_ar { "✅ كل التراكات" } else { "✅ All Tracks" });
    let hb_none_btn = Button::with_label(if is_ar { "🚫 إلغاء الكل" } else { "🚫 None" });
    let hb_scan_btn = Button::with_label(if is_ar { "🔄 فحص التراكات" } else { "🔄 Scan Tracks" });
    hb_bar.append(&hb_all_btn);
    hb_bar.append(&hb_none_btn);
    hb_bar.append(&hb_scan_btn);

    let tracks_list_box = Box::builder().orientation(Orientation::Vertical).spacing(4).build();
    let empty_trk_lbl = Label::new(Some(if is_ar { "اختر ملف فيديو أو اضغط 'فحص التراكات' لعرض الترجمات الداخلية" } else { "Select a video or click 'Scan Tracks' to list internal subtitles" }));
    empty_trk_lbl.set_xalign(0.0);
    tracks_list_box.append(&empty_trk_lbl);
    let track_checks: Rc<RefCell<Vec<gtk::CheckButton>>> = Rc::new(RefCell::new(Vec::new()));

    let sub_row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let sub_sel_btn = Button::with_label(if is_ar { "📂 اختر ملف ترجمة (.ass/.srt)" } else { "📂 Select Subtitle (.ass/.srt)" }); sub_sel_btn.set_hexpand(true);
    let sub_clr_btn = Button::with_label(if is_ar { "🗑️ مسح" } else { "🗑️ Clear" });
    sub_row.append(&sub_sel_btn); sub_row.append(&sub_clr_btn);
    let sub_en = Entry::builder().placeholder_text(if is_ar { "مسار الترجمة الخارجية..." } else { "External subtitle path..." }).build();
    let sub_mode = DropDown::new(Some(StringList::new(&if is_ar { vec!["بدون ترجمة خارجية","حرق الترجمة (Hardsub)","مسار منفصل (Softsub)"] } else { vec!["No external subtitle","Burn-in (Hardsub)","Soft Subtitle (Softsub)"] })), gtk::Expression::NONE);
    let sub_size = DropDown::new(Some(StringList::new(&if is_ar { vec!["حجم الخط: افتراضي (تنسيق الملف)","16","20","24","28","34"] } else { vec!["Size: Default (File Style)","16","20","24","28","34"] })), gtk::Expression::NONE);
    let sub_col = DropDown::new(Some(StringList::new(&if is_ar { vec!["لون الخط: افتراضي (تنسيق الملف)","أبيض","أصفر","سماوي","أخضر"] } else { vec!["Color: Default (File Style)","White","Yellow","Cyan","Lime"] })), gtk::Expression::NONE);
    let sub_bg = DropDown::new(Some(StringList::new(&if is_ar { vec!["خلفية النص: افتراضي (تنسيق الملف)","إطار + ظل","صندوق أسود","إطار سميك"] } else { vec!["BG: Default (File Style)","Outline + Shadow","Black Box","Thick Outline"] })), gtk::Expression::NONE);
    let sub_delay = Entry::builder().placeholder_text(if is_ar { "تقديم/تأخير بالثواني (1.5 أو -2)" } else { "Offset in sec (1.5 or -2)" }).build();
    let sub_lang = Entry::builder().placeholder_text(if is_ar { "رمز اللغة (ara / eng)" } else { "Language code (ara / eng)" }).build();
    sub_box.append(&hb_bar);
    sub_box.append(&tracks_list_box); sub_box.append(&sub_row); sub_box.append(&sub_en); sub_box.append(&sub_mode); sub_box.append(&sub_size); sub_box.append(&sub_col); sub_box.append(&sub_bg); sub_box.append(&sub_delay); sub_box.append(&sub_lang);
    view_stack.add_titled(&wrap_in_scroll(&sub_box), Some("subtitles"), if is_ar { "الترجمة" } else { "Subtitles" });
    let scan_tracks_fn: Rc<dyn Fn()> = {
        let (tlb, tcs, sel_f) = (tracks_list_box.clone(), track_checks.clone(), selected_file.clone());
        Rc::new(move || {
            while let Some(child) = tlb.first_child() { tlb.remove(&child); }
            tcs.borrow_mut().clear();
            if let Some(ref path) = *sel_f.borrow() {
                let trs = probe_sub_tracks(path, is_arabic_system());
                let real_tracks: Vec<&String> = trs.iter().skip(3).collect();
                if real_tracks.is_empty() {
                    let lbl = Label::new(Some(if is_arabic_system() { "لا توجد مسارات ترجمة داخلية في هذا الفيديو" } else { "No internal subtitle tracks found in this video" }));
                    lbl.set_xalign(0.0);
                    tlb.append(&lbl);
                } else {
                    for tr in real_tracks {
                        let cb = gtk::CheckButton::with_label(tr);
                        cb.set_active(false);
                        tlb.append(&cb);
                        tcs.borrow_mut().push(cb);
                    }
                }
            }
        })
    };

    let sc_on_change = scan_tracks_fn.clone();
    output_name_entry.connect_changed(move |_| { sc_on_change(); });
    let sc_on_btn = scan_tracks_fn.clone();
    hb_scan_btn.connect_clicked(move |_| { sc_on_btn(); });

    let tcs_all = track_checks.clone();
    hb_all_btn.connect_clicked(move |_| {
        for cb in tcs_all.borrow().iter() { cb.set_active(true); }
    });
    let tcs_none = track_checks.clone();
    hb_none_btn.connect_clicked(move |_| {
        for cb in tcs_none.borrow().iter() { cb.set_active(false); }
    });

    // --- التبويب 4: الفلاتر ---
    let filters_box = make_tab_box();
    let vf_row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let vf_dropdown = make_dropdown(&caps.video_filters, true);
    vf_dropdown.set_hexpand(true);
    let vf_help_btn = Button::with_label((if is_arabic_system() { "ℹ️ شرح الفلتر" } else { "ℹ️ Filter Help" }));
    let add_vf_btn = Button::with_label((if is_arabic_system() { "➕ إضافة" } else { "➕ Add" }));
    vf_row.append(&vf_dropdown);
    vf_row.append(&vf_help_btn);
    vf_row.append(&add_vf_btn);

    let vf_chain_entry = Entry::builder()
        .placeholder_text((if is_arabic_system() { "سلسلة فلاتر الفيديو (-vf) مثال: scale=1920:1080,hflip" } else { "Video filter chain (-vf) e.g. scale=1920:1080,hflip" }))
        .build();

    let af_row = Box::builder().orientation(Orientation::Horizontal).spacing(8).build();
    let af_dropdown = make_dropdown(&caps.audio_filters, true);
    af_dropdown.set_hexpand(true);
    let af_help_btn = Button::with_label((if is_arabic_system() { "ℹ️ شرح الفلتر" } else { "ℹ️ Filter Help" }));
    let add_af_btn = Button::with_label((if is_arabic_system() { "➕ إضافة" } else { "➕ Add" }));
    af_row.append(&af_dropdown);
    af_row.append(&af_help_btn);
    af_row.append(&add_af_btn);

    let af_chain_entry = Entry::builder()
        .placeholder_text((if is_arabic_system() { "سلسلة فلاتر الصوت (-af) مثال: volume=1.5,loudnorm" } else { "Audio filter chain (-af) e.g. volume=1.5,loudnorm" }))
        .build();

    let extra_global_args = Entry::builder()
        .placeholder_text((if is_arabic_system() { "وسائط عامة إضافية (مثال: -ss 00:01:00 -t 30)" } else { "Extra global flags (e.g. -ss 00:01:00 -t 30)" }))
        .build();

    let (vf_dd_c, vf_en_c) = (vf_dropdown.clone(), vf_chain_entry.clone());
    add_vf_btn.connect_clicked(move |_| {
        let name = get_dropdown_text(&vf_dd_c, "");
        if !name.is_empty() {
            let cur = vf_en_c.text().to_string();
            let next = if cur.trim().is_empty() { name } else { format!("{},{}", cur, name) };
            vf_en_c.set_text(&next);
        }
    });

    let (af_dd_c, af_en_c) = (af_dropdown.clone(), af_chain_entry.clone());
    add_af_btn.connect_clicked(move |_| {
        let name = get_dropdown_text(&af_dd_c, "");
        if !name.is_empty() {
            let cur = af_en_c.text().to_string();
            let next = if cur.trim().is_empty() { name } else { format!("{},{}", cur, name) };
            af_en_c.set_text(&next);
        }
    });

    filters_box.append(&build_quick_presets_box(&vf_chain_entry));
    filters_box.append(&Label::new(Some((if is_arabic_system() { "فلاتر الفيديو المتاحة:" } else { "Available Video Filters:" }))));
    filters_box.append(&vf_row);
    filters_box.append(&vf_chain_entry);
    filters_box.append(&Label::new(Some((if is_arabic_system() { "فلاتر الصوت المتاحة:" } else { "Available Audio Filters:" }))));
    filters_box.append(&af_row);
    filters_box.append(&af_chain_entry);
    filters_box.append(&Label::new(Some((if is_arabic_system() { "وسائط إضافية حرة:" } else { "Custom Global Flags:" }))));
    filters_box.append(&extra_global_args);
    view_stack.add_titled(&wrap_in_scroll(&filters_box), Some("filters"), (if is_arabic_system() { "الفلاتر والأوامر" } else { "Filters & Flags" }));

    // --- أزرار المساعدة ---
    for (btn, dd, topic) in [
        (&format_help_btn, &format_dropdown, "muxer"),
        (&video_help_btn, &video_dropdown, "encoder"),
        (&audio_help_btn, &audio_dropdown, "encoder"),
        (&vf_help_btn, &vf_dropdown, "filter"),
        (&af_help_btn, &af_dropdown, "filter"),
    ] {
        let win_w = window.downgrade();
        let dd_c = dd.clone();
        btn.connect_clicked(move |_| {
            if let Some(win) = win_w.upgrade() {
                let name = get_dropdown_text(&dd_c, "");
                let help_txt = get_ffmpeg_help(topic, &name);
                show_help_dialog(&win, &format!("{}: {}", if is_ar { "خيارات" } else { "Options" }, name), &help_txt);
            }
        });
    }

    // --- الشريط السفلي ---
    let bottom_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    let cmd_entry = Entry::builder()
        .placeholder_text((if is_arabic_system() { "معاينة أمر ffmpeg الكامل (يمكنك تعديله يدوياً قبل التشغيل)" } else { "Live ffmpeg command preview (editable before running)" }))
        .build();
    let progress_bar = ProgressBar::builder().show_text(true).build();
    let status_label = Label::new(Some((if is_arabic_system() { "جاهز للعمل" } else { "Ready" })));
    status_label.set_wrap(true);

    let buttons_row = Box::builder().orientation(Orientation::Horizontal).spacing(12).build();
    let build_cmd_btn = Button::with_label((if is_arabic_system() { "🔄 تحديث الأمر من الحقول" } else { "🔄 Refresh Command" }));
    let convert_btn = Button::with_label((if is_arabic_system() { "🚀 بدء التحويل" } else { "🚀 Start Encoding" }));
    convert_btn.add_css_class("suggested-action");
    let cancel_btn = Button::with_label((if is_arabic_system() { "🛑 إيقاف" } else { "🛑 Cancel" }));
    cancel_btn.add_css_class("destructive-action");
    cancel_btn.set_sensitive(false);

    buttons_row.append(&build_cmd_btn);
    buttons_row.append(&convert_btn);
    let (ws, es, ms, szs, cls, bgs) = (window.downgrade(), sub_en.clone(), sub_mode.clone(), sub_size.clone(), sub_col.clone(), sub_bg.clone());
    sub_sel_btn.connect_clicked(move |_| { if let Some(w) = ws.upgrade() { let (ec, mc, szc, clc, bgc) = (es.clone(), ms.clone(), szs.clone(), cls.clone(), bgs.clone()); FileDialog::builder().build().open(Some(&w), gio::Cancellable::NONE, move |r| { if let Ok(f) = r { if let Some(p) = f.path() { let ps = p.to_string_lossy().to_string(); ec.set_text(&ps); if mc.selected()==0 { mc.set_selected(1); } if ps.to_lowercase().ends_with(".ass") { szc.set_selected(0); clc.set_selected(0); bgc.set_selected(0); } } } }); } });
    let (ec2, mc2) = (sub_en.clone(), sub_mode.clone()); sub_clr_btn.connect_clicked(move |_| { ec2.set_text(""); mc2.set_selected(0); });
    let open_dir_btn = Button::with_label(if is_ar { "📂 فتح المجلد" } else { "📂 Open Folder" });
    let (so, oo) = (selected_file.clone(), output_name_entry.clone());
    open_dir_btn.connect_clicked(move |_| { let c = oo.text().trim().to_string(); if !c.is_empty() { open_dir(&c); } else if let Some(ref i) = *so.borrow() { open_dir(i); } });

    buttons_row.append(&cancel_btn);
    buttons_row.append(&open_dir_btn);

    bottom_box.append(&Label::new(Some((if is_arabic_system() { "الأمر النهائي (ينفذ ما هو مكتوب هنا):" } else { "Final Command (executes exact text below):" }))));
    bottom_box.append(&cmd_entry);
    bottom_box.append(&progress_bar);
    bottom_box.append(&buttons_row);
    bottom_box.append(&status_label);

    // --- تجميع الوسائط ---
    let build_args = {
        let sel = selected_file.clone();
        let fmt_dd = format_dropdown.clone();
        let out_en = output_name_entry.clone();
        let v_dd = video_dropdown.clone();
        let crf_en = crf_entry.clone();
        let vb_en = v_bitrate_entry.clone();
        let fps_en = fps_entry.clone();
        let pre_dd = preset_dropdown.clone();
        let ev_en = extra_v_args.clone();
        let a_dd = audio_dropdown.clone();
        let ab_en = a_bitrate_entry.clone();
        let ar_en = sample_rate_entry.clone();
        let ac_en = channels_entry.clone();
        let vf_en = vf_chain_entry.clone();
        let af_en = af_chain_entry.clone();
        let eg_en = extra_global_args.clone(); let (px_d, tcs_d, s_en, s_md, s_sz, s_cl, s_bg, s_dl, s_lg) = (pix_dd.clone(), track_checks.clone(), sub_en.clone(), sub_mode.clone(), sub_size.clone(), sub_col.clone(), sub_bg.clone(), sub_delay.clone(), sub_lang.clone());

        move || -> Option<Vec<String>> {
            let input = sel.borrow().clone()?;
            let mut args = vec!["-y".to_string()];

            let eg = eg_en.text().to_string();
            if !eg.trim().is_empty() {
                args.extend(parse_shell_args(&eg));
            }

            args.push("-i".to_string());
            args.push(input.clone());

            let sp = s_en.text().trim().to_string(); let sm = s_md.selected(); let sd = s_dl.text().trim().to_string();
            if sm == 2 && !sp.is_empty() { if !sd.is_empty() { args.extend(["-itsoffset".into(), sd]); } args.extend(["-i".into(), sp.clone()]); }
            let mut vc = get_dropdown_text(&v_dd, "copy");
            if vc == "copy" && (!vf_en.text().trim().is_empty() || (sm == 1 && !sp.is_empty())) { vc = "libx264".into(); }
            if vc.starts_with("none") {
                args.push("-vn".to_string());
            } else {
                let p_mode = detect_preset_mode(&vc);
                args.extend(["-c:v".to_string(), vc.clone()]);

                for (flag, entry) in [("-crf", &crf_en), ("-b:v", &vb_en), ("-r", &fps_en)] {
                    let val = entry.text().trim().to_string();
                    if !val.is_empty() {
                        args.extend([flag.to_string(), val]);
                    }
                }

                let p_raw = get_dropdown_text(&pre_dd, (if is_arabic_system() { "افتراضي" } else { "Default" }));
                if !p_raw.starts_with((if is_arabic_system() { "افتراضي" } else { "Default" })) && !p_raw.starts_with("غير متاح") && !p_raw.starts_with("Default") && !p_raw.starts_with("Not available") {
                    let p_val = p_raw.split_whitespace().next().unwrap_or("").to_string();
                    if !p_val.is_empty() {
                        match p_mode {
                            PresetMode::CpuUsed => args.extend(["-cpu-used".to_string(), p_val]),
                            PresetMode::None => {}
                            _ => args.extend(["-preset".to_string(), p_val]),
                        }
                    }
                }

                let ev = ev_en.text().to_string();
                if !vc.starts_with("copy") && !ev.contains("-pix_fmt") { match px_d.selected() { 0 => args.extend(["-pix_fmt".into(),"yuv420p".into()]), 1 => args.extend(["-pix_fmt".into(),"yuv420p10le".into()]), 2 => args.extend(["-pix_fmt".into(),"yuv420p12le".into()]), _ => {} } }
                if !ev.trim().is_empty() {
                    args.extend(parse_shell_args(&ev));
                }
            }

            let ac = get_dropdown_text(&a_dd, "copy");
            if ac.starts_with("none") {
                args.push("-an".to_string());
            } else {
                args.extend(["-c:a".to_string(), ac]);
                for (flag, entry) in [("-b:a", &ab_en), ("-ar", &ar_en), ("-ac", &ac_en)] {
                    let val = entry.text().trim().to_string();
                    if !val.is_empty() {
                        args.extend([flag.to_string(), val]);
                    }
                }
            }

            let mut vfs = Vec::new();
            if !vf_en.text().trim().is_empty() { vfs.push(vf_en.text().trim().to_string()); }
            if sm == 1 && !sp.is_empty() {
                let mut st = Vec::new();
                if s_sz.selected() > 0 { st.push(format!("FontSize={}", ["","16","20","24","28","34"][s_sz.selected() as usize])); }
                if s_cl.selected() > 0 { st.push(format!("PrimaryColour={}", ["","&H00FFFFFF","&H0000FFFF","&H00FFFF00","&H0000FF00"][s_cl.selected() as usize])); }
                if s_bg.selected() == 1 { st.push("BorderStyle=1,Outline=1.5,Shadow=1".into()); } else if s_bg.selected() == 2 { st.push("BorderStyle=3,BackColour=&H80000000".into()); } else if s_bg.selected() == 3 { st.push("BorderStyle=1,Outline=2.5,Shadow=0".into()); }
                vfs.push(if st.is_empty() { format!("subtitles='{}'", escape_sub(&sp)) } else { format!("subtitles='{}':force_style='{}'", escape_sub(&sp), st.join(",")) });
            }
            if !vfs.is_empty() { args.extend(["-vf".into(), vfs.join(",")]); }
            if !af_en.text().trim().is_empty() { args.extend(["-af".into(), af_en.text().trim().to_string()]); }
            let ext_c = get_dropdown_text(&fmt_dd, "mp4").to_lowercase(); let is_mp4 = ext_c == "mp4" || ext_c == "mov";
            if !["mp3","m4a","flac","wav","opus","ogg"].contains(&ext_c.as_str()) {
                let cbs = tcs_d.borrow();
                let active_idxs: Vec<usize> = cbs.iter().enumerate().filter(|(_, c)| c.is_active()).map(|(i, _)| i).collect();
                if !cbs.is_empty() && !active_idxs.is_empty() {
                    args.extend(["-map".into(), "0:v:0".into(), "-map".into(), "0:a?".into()]);
                    for idx in &active_idxs {
                        args.extend(["-map".into(), format!("0:s:{}", idx)]);
                    }
                    if sm == 2 && !sp.is_empty() { args.extend(["-map".into(), "1:0".into()]); }
                    args.extend(["-c:s".into(), if is_mp4 { "mov_text" } else { "copy" }.into()]);
                } else if sm == 2 && !sp.is_empty() {
                    args.extend(["-map".into(), "0:v:0".into(), "-map".into(), "0:a?".into(), "-map".into(), "1:0".into(), "-c:s".into(), if is_mp4 { "mov_text" } else { "copy" }.into()]);
                }
                let lg = s_lg.text().trim().to_string(); if sm == 2 && !sp.is_empty() && !lg.is_empty() { args.extend(["-metadata:s:s:0".into(), format!("language={}", lg)]); }
            }

            let ext = get_dropdown_text(&fmt_dd, "mp4");
            let custom = out_en.text().trim().to_string();
            let out_path = if custom.is_empty() {
                build_default_output_path(&input, &ext)
            } else {
                custom
            };
            args.push(out_path);
            Some(args)
        }
    };

    // --- تحديث المعاينة الحية تلقائياً ---
    let refresh_preview = {
        let ba = build_args.clone();
        let ce = cmd_entry.clone();
        move || {
            if let Some(a) = ba() {
                ce.set_text(&format_cmd_preview(&a));
            }
        }
    };

    // --- تغيير قائمة Preset تلقائياً عند تغيير ترميز الفيديو ---
    let v_dd_preset = video_dropdown.clone();
    let p_dd_preset = preset_dropdown.clone();
    let p_lbl_preset = preset_label.clone();
    let rp_v = refresh_preview.clone();
    v_dd_preset.connect_selected_notify(move |dd| {
        let codec = get_dropdown_text(dd, "copy");
        let mode = detect_preset_mode(&codec);
        let opts = preset_options_for_mode(&mode);
        p_dd_preset.set_model(Some(&StringList::new(&opts)));
        p_dd_preset.set_selected(0);

        match mode {
            PresetMode::None => {
                p_dd_preset.set_sensitive(false);
                p_lbl_preset.set_text((if is_arabic_system() { "سرعة الضغط (غير مدعومة لهذا الترميز):" } else { "Encoding Speed (Not supported for this codec):" }));
            }
            PresetMode::CpuUsed => {
                p_dd_preset.set_sensitive(true);
                p_lbl_preset.set_text((if is_arabic_system() { "سرعة الضغط (-cpu-used لـ VPX/AV1):" } else { "Encoding Speed (-cpu-used for VPX/AV1):" }));
            }
            PresetMode::SvtAv1 => {
                p_dd_preset.set_sensitive(true);
                p_lbl_preset.set_text((if is_arabic_system() { "سرعة الضغط (-preset 0..13 لـ SVT-AV1):" } else { "Encoding Speed (-preset 0..13 for SVT-AV1):" }));
            }
            PresetMode::Nvenc => {
                p_dd_preset.set_sensitive(true);
                p_lbl_preset.set_text((if is_arabic_system() { "سرعة الضغط (-preset p1..p7 لـ NVENC):" } else { "Encoding Speed (-preset p1..p7 for NVENC):" }));
            }
            PresetMode::StandardPreset => {
                p_dd_preset.set_sensitive(true);
                p_lbl_preset.set_text((if is_arabic_system() { "سرعة الضغط (-preset):" } else { "Encoding Speed (-preset):" }));
            }
        }
        rp_v();
    });

    for dd in [&format_dropdown, &preset_dropdown, &audio_dropdown, &pix_dd, &sub_mode, &sub_size, &sub_col, &sub_bg] {
        let rp = refresh_preview.clone();
        dd.connect_selected_notify(move |_| rp());
    }
    for en in [
        &output_name_entry,
        &crf_entry,
        &v_bitrate_entry,
        &fps_entry,
        &extra_v_args,
        &a_bitrate_entry,
        &sample_rate_entry,
        &channels_entry,
        &vf_chain_entry,
        &af_chain_entry,
        &extra_global_args, &sub_en, &sub_delay, &sub_lang,
    ] {
        let rp = refresh_preview.clone();
        en.connect_changed(move |_| rp());
    }

    // --- اختيار الملف والسحب والإفلات ---
    let set_file_handler = {
        let file_lbl = file_label.clone();
        let sel_file = selected_file.clone();
        let info_lbl = media_info_label.clone();
        let pbox = preview_slot.clone();
        let sc_fn = scan_tracks_fn.clone();
        let rp = refresh_preview.clone();
        move |p_str: String| {
            file_lbl.set_text(&format!("{}: {}", if is_ar { "المصدر" } else { "Source" }, p_str));
            let mut out_lines = Vec::new();
            if let Ok(meta) = std::fs::metadata(&p_str) { out_lines.push(format!("• 📁 <b>Size:</b> {:.1} MB", meta.len() as f64 / 1048576.0)); }
            if let Ok(o) = new_std_cmd("ffprobe").args(["-v","error","-show_entries","format=duration:stream=index,codec_type,codec_name,width,height,r_frame_rate,sample_rate,channels,bit_rate:stream_tags=language,title","-of","default=noprint_wrappers=1",&p_str]).output() {
                let mut streams = Vec::new(); let mut cur = std::collections::HashMap::new(); let mut in_st = false;
                for ln in String::from_utf8_lossy(&o.stdout).lines() {
                    if let Some((k, v)) = ln.split_once('=') {
                        if k == "duration" { if let Ok(d) = v.parse::<f64>() { out_lines.push(format!("• ⏱️ <b>Duration:</b> {:02}:{:02}", (d as u64)/60, (d as u64)%60)); } }
                        else if k == "index" { if in_st && !cur.is_empty() { streams.push(cur.clone()); cur.clear(); } in_st = true; }
                        else if in_st { cur.insert(k.to_string(), v.to_string()); }
                    }
                }
                if in_st && !cur.is_empty() { streams.push(cur); }
                let mut subs = Vec::new();
                for st in streams {
                    let tp = st.get("codec_type").map(|x| x.as_str()).unwrap_or("");
                    let cd = st.get("codec_name").map(|x| x.to_uppercase()).unwrap_or_default();
                    if tp == "video" {
                        let w = st.get("width").cloned().unwrap_or_default(); let h = st.get("height").cloned().unwrap_or_default();
                        let mut fps_s = String::new();
                        if let Some((n, d)) = st.get("r_frame_rate").and_then(|x| x.split_once('/')) { if let (Ok(nf), Ok(df)) = (n.parse::<f64>(), d.parse::<f64>()) { if df > 0.0 { fps_s = format!(" | {:.2} fps", nf/df); } } }
                        out_lines.push(format!("\n• 🎬 <b>Video</b>\n   • {} | {}×{}{}", cd, w, h, fps_s));
                    } else if tp == "audio" {
                        let lg = st.get("TAG:language").cloned().unwrap_or_default(); let sr = st.get("sample_rate").cloned().unwrap_or_default();
                        let ch = st.get("channels").cloned().unwrap_or_default();
                        let br = st.get("bit_rate").and_then(|x| x.parse::<u64>().ok()).map(|b| format!(" | {} kbps", b / 1000)).unwrap_or_default();
                        out_lines.push(format!("\n• 🔊 <b>Audio ({})</b>\n   • {} | {} Hz | {} ch{}", lg, cd, sr, ch, br));
                    } else if tp == "subtitle" {
                        let lg = st.get("TAG:language").cloned().unwrap_or_else(|| "und".into()); let ti = st.get("TAG:title").cloned().unwrap_or_default();
                        subs.push(format!("{} {} [{}]", gtk::glib::markup_escape_text(&lg), gtk::glib::markup_escape_text(&ti), cd));
                    }
                }
                if !subs.is_empty() { out_lines.push(format!("\n• 💬 <b>Subtitles ({})</b>", subs.len())); for sb in subs { out_lines.push(format!("   • {}", sb)); } }
            }
            info_lbl.set_markup(&format!("<span size='large'>{}</span>", out_lines.join("\n")));
            sc_fn();
            let vpath_owned = p_str.clone();
            let pbox_c = pbox.clone();
            while let Some(c) = pbox_c.first_child() { pbox_c.remove(&c); }
            let loading = Label::new(Some(if is_ar { "⏳ جاري استخراج لقطات المعاينة..." } else { "⏳ Generating preview frames..." }));
            loading.set_vexpand(true);
            pbox_c.append(&loading);
            let (tx, rx) = std::sync::mpsc::channel::<Vec<PathBuf>>();
            std::thread::spawn(move || { let _ = tx.send(extract_video_thumbnails(&vpath_owned)); });
            glib::timeout_add_local(std::time::Duration::from_millis(120), move || {
                match rx.try_recv() {
                    Ok(imgs) => {
                        while let Some(c) = pbox_c.first_child() { pbox_c.remove(&c); }
                        if imgs.is_empty() {
                            let no_img = Label::new(Some(if is_ar { "🎵 ملف صوتي أو بدون إطارات" } else { "🎵 Audio or no frames" }));
                            no_img.set_vexpand(true);
                            pbox_c.append(&no_img);
                        } else {
                            let grid = gtk::Grid::builder().row_spacing(6).column_spacing(6).row_homogeneous(true).column_homogeneous(true).vexpand(true).hexpand(true).margin_top(8).margin_bottom(8).margin_start(8).margin_end(8).build();
                            for (i, img_p) in imgs.iter().enumerate() {
                                let pic = gtk::Picture::for_file(&gio::File::for_path(img_p));
                                pic.set_can_shrink(true); pic.set_hexpand(true); pic.set_vexpand(true);
                                grid.attach(&pic, (i % 2) as i32, (i / 2) as i32, 1, 1);
                            }
                            pbox_c.append(&grid);
                        }
                        glib::ControlFlow::Break
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(_) => glib::ControlFlow::Break,
                }
            });
            *sel_file.borrow_mut() = Some(p_str);
            rp();
        }
    };

    let drop_target = gtk::DropTarget::new(gio::File::static_type(), gdk::DragAction::COPY);
    let set_file_dnd = set_file_handler.clone();
    drop_target.connect_drop(move |_, value, _, _| {
        if let Ok(file) = value.get::<gio::File>() {
            if let Some(path) = file.path() {
                set_file_dnd(path.to_string_lossy().to_string());
                return true;
            }
        }
        false
    });
    window.add_controller(drop_target);

    let window_weak = window.downgrade();
    let set_file_btn = set_file_handler.clone();
    select_btn.connect_clicked(move |_| {
        if let Some(win) = window_weak.upgrade() {
            let dialog = FileDialog::builder().title((if is_arabic_system() { "اختر ملفاً" } else { "Select Media File" })).build();
            let setter = set_file_btn.clone();
            dialog.open(Some(&win), gio::Cancellable::NONE, move |res| {
                if let Ok(file) = res {
                    if let Some(path) = file.path() {
                        setter(path.to_string_lossy().to_string());
                    }
                }
            });
        }
    });

    let (b_args_c, cmd_en_c, st_lbl_c) =
        (build_args.clone(), cmd_entry.clone(), status_label.clone());
    build_cmd_btn.connect_clicked(move |_| {
        if let Some(args) = b_args_c() {
            cmd_en_c.set_text(&format_cmd_preview(&args));
            st_lbl_c.set_text((if is_arabic_system() { "تم تحديث الأمر من الحقول." } else { "Command refreshed from fields." }));
        } else {
            st_lbl_c.set_text((if is_arabic_system() { "⚠️ اختر ملف مصدر أولاً!" } else { "⚠️ Please select a source file first!" }));
        }
    });

    let running_cancel = running_child.clone();
    let st_lbl_cancel = status_label.clone();
    cancel_btn.connect_clicked(move |_| {
        if let Ok(mut guard) = running_cancel.lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.kill();
                st_lbl_cancel.set_text((if is_arabic_system() { "🛑 تم إيقاف التحويل." } else { "🛑 Encoding cancelled." }));
            }
        }
    });

    let (cmd_en_r, st_lbl_r, p_bar_r, c_btn_r, x_btn_r, sel_r, run_r) = (
        cmd_entry.clone(),
        status_label.clone(),
        progress_bar.clone(),
        convert_btn.clone(),
        cancel_btn.clone(),
        selected_file.clone(),
        running_child.clone(),
    );

    convert_btn.connect_clicked(move |_| {
        let manual_cmd = cmd_en_r.text().trim().to_string();
        let args: Vec<String> = if manual_cmd.starts_with("ffmpeg ") {
            parse_shell_args(manual_cmd.strip_prefix("ffmpeg ").unwrap_or(""))
        } else {
            match build_args() {
                Some(a) => {
                    cmd_en_r.set_text(&format_cmd_preview(&a));
                    a
                }
                None => {
                    st_lbl_r.set_text((if is_arabic_system() { "⚠️ اختر ملف مصدر أولاً!" } else { "⚠️ Please select a source file first!" }));
                    return;
                }
            }
        };

        let mut exec_args = vec![
            "-progress".to_string(),
            "pipe:1".to_string(),
            "-nostats".to_string(),
        ];
        exec_args.extend(args);

        let total_dur = sel_r
            .borrow()
            .as_ref()
            .map(|p| get_media_duration(p))
            .unwrap_or(0.0);

        p_bar_r.set_fraction(0.0);
        st_lbl_r.set_text((if is_arabic_system() { "⏳ جاري المعالجة..." } else { "⏳ Processing..." }));
        c_btn_r.set_sensitive(false);
        x_btn_r.set_sensitive(true);

        let (tx, rx) = mpsc::channel::<ProgressMsg>();
        let run_thread = run_r.clone();

        std::thread::spawn(move || {
            let mut child = match new_std_cmd("ffmpeg")
                .args(&exec_args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ProgressMsg::Done(Err(format!("{}: {}", if is_arabic_system() { "فشل التشغيل" } else { "Launch failed" }, e))));
                    return;
                }
            };

            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            if let Ok(mut guard) = run_thread.lock() {
                *guard = Some(child);
            }

            if let Some(out) = stdout {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    if let Some(ms_str) = line.strip_prefix("out_time_us=") {
                        if let Ok(us) = ms_str.trim().parse::<f64>() {
                            let cur = us / 1_000_000.0;
                            if total_dur > 0.0 {
                                let ratio = (cur / total_dur).clamp(0.0, 1.0);
                                let msg = format!(
                                    "التقدم: {:.1}% ({:.1}s / {:.1}s)",
                                    ratio * 100.0, cur, total_dur
                                );
                                let _ = tx.send(ProgressMsg::Progress(ratio, msg));
                            }
                        }
                    }
                }
            }

            let wait_res = if let Ok(mut guard) = run_thread.lock() {
                guard.take().map(|mut c| c.wait())
            } else {
                None
            };

            match wait_res {
                Some(Ok(status)) if status.success() => {
                    let _ = tx.send(ProgressMsg::Done(Ok((if is_arabic_system() { "✅ اكتمل التحويل بنجاح!" } else { "✅ Encoding completed successfully!" }).to_string())));
                }
                Some(Ok(_)) => {
                    let mut err_str = String::new();
                    if let Some(mut err) = stderr {
                        let _ = err.read_to_string(&mut err_str);
                    }
                    let last: Vec<&str> = err_str.lines().rev().take(4).collect();
                    let joined = last.into_iter().rev().collect::<Vec<&str>>().join("
");
                    let _ = tx.send(ProgressMsg::Done(Err(format!("❌ توقف أو خطأ:\n{}", joined))));
                }
                _ => {
                    let _ = tx.send(ProgressMsg::Done(Err((if is_arabic_system() { "❌ تعذر إتمام العملية." } else { "❌ Operation failed." }).to_string())));
                }
            }
        });

        let (pb, sl, cb, xb) = (
            p_bar_r.clone(),
            st_lbl_r.clone(),
            c_btn_r.clone(),
            x_btn_r.clone(),
        );
        glib::timeout_add_local(Duration::from_millis(50), move || {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ProgressMsg::Progress(r, t) => {
                        pb.set_fraction(r);
                        sl.set_text(&t);
                    }
                    ProgressMsg::Done(res) => {
                        cb.set_sensitive(true);
                        xb.set_sensitive(false);
                        match res {
                            Ok(ok) => {
                                pb.set_fraction(1.0);
                                sl.set_text(&ok);
                            }
                            Err(err) => sl.set_text(&err),
                        }
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    });

    let main_vbox = Box::builder().orientation(Orientation::Vertical).build();
    view_stack.set_vexpand(true);
    main_vbox.append(&view_stack);
    main_vbox.append(&bottom_box);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&main_vbox));

    window.set_content(Some(&toolbar_view));
    window.present();
}
