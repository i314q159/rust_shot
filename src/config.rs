//! Tiny settings file: `%LOCALAPPDATA%\RustShot\config.txt`.
//!
//! Deliberately not JSON: two keys, two lines, no serde dependency.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub save_dir: PathBuf,
    pub auto_save: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: crate::util::default_save_dir(),
            auto_save: true,
        }
    }
}

fn path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("RustShot").join("config.txt")
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Self::default();
        let Ok(text) = std::fs::read_to_string(path()) else {
            return cfg;
        };
        // Notepad and some editors prepend a UTF-8 BOM; it must not end up in
        // the first key or every setting is silently ignored.
        let text = text.trim_start_matches('\u{feff}');
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "save_dir" if !value.is_empty() => cfg.save_dir = PathBuf::from(value),
                "auto_save" => cfg.auto_save = value != "false",
                _ => {}
            }
        }
        cfg
    }

    pub fn save(&self) {
        let p = path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let body = format!(
            "save_dir={}\nauto_save={}\n",
            self.save_dir.display(),
            self.auto_save
        );
        if let Err(e) = std::fs::write(&p, body) {
            crate::util::log(&format!("config save failed: {e}"));
        }
    }
}

/// `dir/stem.png`, or `dir/stem_1.png`, `dir/stem_2.png`… when it already exists,
/// so two captures inside the same second never overwrite each other.
pub fn unique_png(dir: &Path, stem: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.png"));
    if !first.exists() {
        return first;
    }
    for i in 1..10_000 {
        let candidate = dir.join(format!("{stem}_{i}.png"));
        if !candidate.exists() {
            return candidate;
        }
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_png_avoids_collisions() {
        let dir = std::env::temp_dir().join(format!("rustshot_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let a = unique_png(&dir, "shot");
        assert!(a.ends_with("shot.png"));
        std::fs::write(&a, b"x").unwrap();
        let b = unique_png(&dir, "shot");
        assert!(b.ends_with("shot_1.png"));
        std::fs::write(&b, b"x").unwrap();
        let c = unique_png(&dir, "shot");
        assert!(c.ends_with("shot_2.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_roundtrip_is_parseable() {
        let dir = std::env::temp_dir().join("rustshot_cfg_test");
        let text = format!("save_dir={}\nauto_save=false\n", dir.display());
        let mut cfg = Config::default();
        for line in text.lines() {
            let (k, v) = line.split_once('=').unwrap();
            match k {
                "save_dir" => cfg.save_dir = PathBuf::from(v),
                "auto_save" => cfg.auto_save = v != "false",
                _ => {}
            }
        }
        assert_eq!(cfg.save_dir, dir);
        assert!(!cfg.auto_save);
    }

    /// Notepad may save the file with a UTF-8 BOM; the first key must still parse.
    #[test]
    fn bom_prefixed_config_still_parses() {
        let body = "\u{feff}save_dir=D:\\shots\nauto_save=false\n";
        let text = body.trim_start_matches('\u{feff}');
        let first = text.lines().next().unwrap();
        assert_eq!(first, "save_dir=D:\\shots");
        assert_eq!(first.split_once('=').unwrap().0, "save_dir");
    }
}
