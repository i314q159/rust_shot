//! Tiny portable settings file: `config.txt` next to the executable.
//!
//! Deliberately not JSON: two keys, two lines, no serde dependency.
//! `save_dir` may use `%USERPROFILE%`-style variables so the folder stays valid
//! when the tool is moved between machines or user accounts.

use std::path::{Path, PathBuf};

/// Name of the settings file dropped next to `rust_shot.exe`.
pub const CONFIG_FILE: &str = "config.txt";
/// Written into `save_dir` when the user resets the folder.
pub const DEFAULT_SAVE_DIR_PARAM: &str = r"%USERPROFILE%\Pictures\RustShot";

#[derive(Clone, Debug)]
pub struct Config {
    pub save_dir: PathBuf,
    pub auto_save: bool,
    /// Top bar theme choice: system / light / dark.
    pub theme: crate::theme::Preference,
    /// UI language: system / zh / en.
    pub lang: crate::i18n::Preference,
    /// Update check: only notify, or install silently once found.
    pub auto_update: crate::update::AutoUpdate,
    /// Proxy for the update check / download.
    pub proxy: crate::proxy::Proxy,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: crate::util::default_save_dir(),
            auto_save: true,
            theme: crate::theme::Preference::System,
            lang: crate::i18n::Preference::System,
            auto_update: crate::update::AutoUpdate::default(),
            proxy: crate::proxy::Proxy::default(),
        }
    }
}

/// `…\<exe dir>\config.txt` — the portable location shown to the user.
pub fn path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(CONFIG_FILE)))
}

/// Older releases kept the file under `%LOCALAPPDATA%\RustShot`.
fn legacy_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("RustShot").join(CONFIG_FILE)
}

/// Expand `%VAR%` (and `$VAR`-free) references in a hand-edited path.
pub fn expand_env(value: &str) -> PathBuf {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match std::env::var_os(name) {
                    Some(v) if !name.is_empty() => out.push_str(&v.to_string_lossy()),
                    // Unknown variable: keep the text as typed.
                    _ => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                out.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    PathBuf::from(out)
}

impl Config {
    /// Reads the file next to the exe, migrating an old LocalAppData copy once.
    pub fn load() -> Self {
        let mut cfg = Self::default();
        let portable = path();
        let source = match &portable {
            Some(p) if p.exists() => Some(p.clone()),
            _ => {
                let old = legacy_path();
                old.exists().then_some(old)
            }
        };
        let Some(file) = source else {
            return cfg;
        };
        let Ok(text) = std::fs::read_to_string(&file) else {
            return cfg;
        };
        // Notepad and some editors prepend a UTF-8 BOM; it must not end up in
        // the first key or every setting is silently ignored.
        let text = text.trim_start_matches('\u{feff}');
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "save_dir" if !value.is_empty() => cfg.save_dir = expand_env(value),
                "auto_save" => cfg.auto_save = value != "false",
                "theme" => cfg.theme = crate::theme::Preference::parse(value),
                "lang" => cfg.lang = crate::i18n::Preference::parse(value),
                "auto_update" => cfg.auto_update = crate::update::AutoUpdate::parse(value),
                "proxy" => cfg.proxy = crate::proxy::Proxy::parse(value),
                _ => {}
            }
        }
        // A legacy copy is left in place: `save()` writes next to the exe, and
        // rewriting on every read would fight with the UI's own saves.
        cfg
    }

    pub fn save(&self) {
        // The default folder is written as a parameter so the file keeps working
        // for another user account / another machine.
        let dir = if self.save_dir == crate::util::default_save_dir() {
            DEFAULT_SAVE_DIR_PARAM.to_owned()
        } else {
            self.save_dir.display().to_string()
        };
        let header = crate::i18n::tr(
            "# RustShot 配置（保存后重启生效；save_dir 支持 %USERPROFILE% 这类环境变量）\n\
             # theme: system | light | dark   lang: system | zh | en\n",
            "# RustShot config (restart to apply; save_dir accepts %USERPROFILE% variables)\n\
             # theme: system | light | dark   lang: system | zh | en\n",
        );
        let body = format!(
            "{header}save_dir={dir}\nauto_save={}\ntheme={}\nlang={}\nauto_update={}\nproxy={}\n",
            self.auto_save,
            self.theme.key(),
            self.lang.key(),
            self.auto_update.key(),
            self.proxy.key()
        );
        let primary = path();
        if let Some(p) = &primary
            && std::fs::write(p, &body).is_ok()
        {
            return;
        }
        // Read-only install folder (Program Files, network share…): keep settings
        // in LocalAppData instead of losing them.
        let fallback = legacy_path();
        if let Some(dir) = fallback.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&fallback, body) {
            Ok(()) => crate::util::log(&format!("config saved to fallback {}", fallback.display())),
            Err(e) => crate::util::log(&format!("config save failed: {e}")),
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

    #[test]
    fn default_save_dir_is_a_parameter() {
        assert_eq!(
            expand_env(DEFAULT_SAVE_DIR_PARAM),
            crate::util::default_save_dir()
        );
    }

    #[test]
    fn expand_env_keeps_unknown_variables_and_plain_paths() {
        let literal = r"C:\Shots\RustShot";
        assert_eq!(expand_env(literal), PathBuf::from(literal));
        let unknown = expand_env("%RUSTSHOT_NO_SUCH_VAR%\\shots");
        assert_eq!(unknown, PathBuf::from("%RUSTSHOT_NO_SUCH_VAR%\\shots"));
    }

    #[test]
    fn config_path_sits_next_to_the_exe() {
        let p = path().expect("test binary has a path");
        assert_eq!(p.file_name().unwrap(), CONFIG_FILE);
        assert_eq!(p.parent(), std::env::current_exe().unwrap().parent());
    }
}
