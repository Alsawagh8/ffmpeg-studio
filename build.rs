fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/ffmpeg-studio.ico");
        if std::path::Path::new("C:\\msys64\\ucrt64\\bin\\windres.exe").exists() {
            res.set_windres_path("C:\\msys64\\ucrt64\\bin\\windres.exe");
        }
        res.compile().expect("Failed to compile Windows resources");
    }
}
