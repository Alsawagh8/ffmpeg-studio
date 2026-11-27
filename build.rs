fn main() {
    #[cfg(windows)]
    {
        use std::env;
        use std::fs;
        use std::path::PathBuf;
        use std::process::Command;

        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        let rc_path = out_dir.join("app.rc");
        let res_obj = out_dir.join("app_res.o");

        let ico_abs = fs::canonicalize("assets/ffmpeg-studio.ico")
            .expect("assets/ffmpeg-studio.ico not found");
        let ico_str = ico_abs.to_string_lossy().replace('\\', "/");

        fs::write(&rc_path, format!("1 ICON \"{}\"\n", ico_str)).unwrap();

        let status = Command::new("windres")
            .args([
                rc_path.to_str().unwrap(),
                "-O",
                "coff",
                "-o",
                res_obj.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to execute windres");

        assert!(status.success(), "windres failed to compile icon resource");
        println!("cargo:rustc-link-arg={}", res_obj.display());
    }
}
