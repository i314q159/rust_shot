pub(crate) mod apply;
pub(crate) mod checksum;
pub(crate) mod json;
pub(crate) mod net;
pub(crate) mod sha256;
pub(crate) mod version;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

pub use checksum::{expected_sha, sha256_file};
pub use json::parse_release;
pub use net::{download, fetch_latest, open_page};
pub use version::{human_size, is_newer, normalize_version, tag_from_release_url};

pub const REPO: &str = "i314q159/rust_shot";

pub const RELEASES_URL: &str = "https://github.com/i314q159/rust_shot/releases/latest";

const API_URL: &str = "https://api.github.com/repos/i314q159/rust_shot/releases/latest";

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,

    Checking,

    Latest(String),

    Newer(Release),
    Failed(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoUpdate {
    #[default]
    Notice,

    Silent,
}

impl AutoUpdate {
    pub fn key(self) -> &'static str {
        match self {
            Self::Notice => "notice",
            Self::Silent => "silent",
        }
    }

    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "silent" | "auto" | "yes" => Self::Silent,
            _ => Self::Notice,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub tag: String,
    pub name: String,
    pub url: String,

    pub published: String,
    pub notes: String,
}

pub fn check_async() -> Receiver<Status> {
    let (tx, rx) = channel();
    let _ = std::thread::Builder::new()
        .name("rustshot-update".into())
        .spawn(move || {
            let status = evaluate(fetch_latest());
            crate::platform::util::log(&format!("update check: {status:?}"));
            let _ = tx.send(status);
            crate::app::request_repaint();
        });
    rx
}

fn evaluate(result: Result<Release, String>) -> Status {
    match result {
        Ok(rel) if is_newer(&rel.version, CURRENT) => Status::Newer(rel),
        Ok(rel) => Status::Latest(rel.version),
        Err(e) => Status::Failed(e),
    }
}

pub fn asset_url(version: &str, name: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/v{version}/{name}")
}

pub fn setup_name(version: &str) -> String {
    format!("RustShot-Setup-{version}.exe")
}
pub const SUMS_NAME: &str = "SHA256SUMS.txt";

pub fn update_dir() -> PathBuf {
    std::env::temp_dir().join("RustShot").join("update")
}

pub fn installed_copy_at(exe: &Path) -> bool {
    exe.parent()
        .map(|dir| dir.join("unins000.exe").exists())
        .unwrap_or(false)
}
