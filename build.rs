

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
        
        
        if let Err(e) = res.compile() {
            println!("cargo:warning=exe icon skipped: {e}");
        }
    }
}
