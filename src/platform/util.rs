

use std::io::Write;
use std::path::PathBuf;


pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}


pub fn timestamp() -> String {
    let st = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    )
}


pub fn default_save_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\"));
    home.join("Pictures").join("RustShot")
}


pub fn log_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("RustShot").join("rustshot.log")
}


pub fn log(msg: &str) {
    let path = log_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(meta) = std::fs::metadata(&path)
        && meta.len() > 512 * 1024
    {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "[{}] {}", timestamp(), msg);
    }
}


pub fn install_panic_logger() {
    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "?".to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        log(&format!("PANIC at {location}: {payload}"));
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_is_nul_terminated() {
        let w = wide("ab");
        assert_eq!(w, vec![97, 98, 0]);
    }

    #[test]
    fn timestamp_has_expected_shape() {
        let ts = timestamp();
        assert_eq!(ts.len(), 15, "yyyymmdd_hhmmss, got {ts}");
        assert_eq!(&ts[8..9], "_");
        assert!(ts.chars().filter(|c| c.is_ascii_digit()).count() == 14);
    }

    #[test]
    fn log_path_lives_under_localappdata() {
        assert!(log_path().ends_with("config.txt".replace("config.txt", "rustshot.log")));
    }
}
