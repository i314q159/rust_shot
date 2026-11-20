//! Embeds the drawn icon into the executable's resources (Explorer / taskbar).

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/rustshot.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon/rustshot.ico");
        res.set("FileDescription", "RustShot 截图工具");
        res.set("ProductName", "RustShot");
        res.set("OriginalFilename", "rust_shot.exe");
        res.set_language(0x0804);
        // A missing resource compiler must not break the build; the app still
        // runs, it just falls back to the default shell icon.
        if let Err(e) = res.compile() {
            println!("cargo:warning=exe icon skipped: {e}");
        }
    }
}
