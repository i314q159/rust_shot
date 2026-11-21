pub(crate) mod fonts;
pub(crate) mod i18n;
pub(crate) mod proxy;

use crate::config::i18n::strings;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "config.toml";

const LEGACY_FILE: &str = "config.txt";

pub const DEFAULT_SAVE_DIR_PARAM: &str = r"%USERPROFILE%\Pictures\RustShot";

#[derive(Clone, Debug)]
pub struct Config {
    pub save_dir: PathBuf,
    pub auto_save: bool,

    pub theme: crate::theme::Preference,

    pub lang: crate::config::i18n::Preference,

    pub auto_update: crate::update::AutoUpdate,

    pub proxy: crate::config::proxy::Proxy,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_dir: crate::platform::util::default_save_dir(),
            auto_save: true,
            theme: crate::theme::Preference::System,
            lang: crate::config::i18n::Preference::System,
            auto_update: crate::update::AutoUpdate::default(),
            proxy: crate::config::proxy::Proxy::default(),
        }
    }
}

pub fn path() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(CONFIG_FILE)))
}

fn fallback_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("RustShot")
}

fn fallback_path() -> PathBuf {
    fallback_dir().join(CONFIG_FILE)
}

fn legacy_paths() -> [PathBuf; 2] {
    let portable = path()
        .and_then(|p| p.parent().map(|dir| dir.join(LEGACY_FILE)))
        .unwrap_or_else(|| PathBuf::from(LEGACY_FILE));
    [portable, fallback_dir().join(LEGACY_FILE)]
}

pub fn exists() -> bool {
    [path(), Some(fallback_path())]
        .into_iter()
        .flatten()
        .any(|p| p.exists())
}

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
    pub fn load() -> Self {
        for file in [path(), Some(fallback_path())].into_iter().flatten() {
            if let Ok(text) = std::fs::read_to_string(&file) {
                return Self::parse(&text);
            }
        }
        for old in legacy_paths() {
            let Ok(text) = std::fs::read_to_string(&old) else {
                continue;
            };
            let cfg = Self::parse(&text);

            cfg.save();
            let backup = old.with_extension("txt.bak");
            if std::fs::rename(&old, &backup).is_ok() {
                crate::platform::util::log(&format!(
                    "config migrated {} -> {CONFIG_FILE} (backup {})",
                    old.display(),
                    backup.display()
                ));
            }
            return cfg;
        }
        Self::default()
    }

    fn parse(text: &str) -> Self {
        let mut cfg = Self::default();
        for line in lines_of(text) {
            let Some((key, raw)) = line.split_once('=') else {
                continue;
            };
            let Some(value) = unquote(raw.trim()) else {
                continue;
            };
            match key.trim() {
                "save_dir" => {
                    if !value.is_empty() {
                        cfg.save_dir = expand_env(&value);
                    }
                }
                "auto_save" => {
                    cfg.auto_save = !matches!(
                        value.trim().to_ascii_lowercase().as_str(),
                        "false" | "0" | "no" | "off"
                    );
                }
                "theme" => cfg.theme = crate::theme::Preference::parse(&value),
                "lang" => cfg.lang = crate::config::i18n::Preference::parse(&value),
                "auto_update" => cfg.auto_update = crate::update::AutoUpdate::parse(&value),
                "proxy" => cfg.proxy = crate::config::proxy::Proxy::parse(&value),
                _ => {}
            }
        }
        cfg
    }

    fn render(&self) -> String {
        let dir = if self.save_dir == crate::platform::util::default_save_dir() {
            DEFAULT_SAVE_DIR_PARAM.to_owned()
        } else {
            self.save_dir.display().to_string()
        };
        let header = crate::config::i18n::tr(strings::CONFIG_HEADER);
        format!(
            "{header}\nsave_dir = {}\nauto_save = {}\ntheme = {}\nlang = {}\nauto_update = {}\nproxy = {}\n",
            toml_string(&dir),
            self.auto_save,
            toml_string(self.theme.key()),
            toml_string(self.lang.key()),
            toml_string(self.auto_update.key()),
            toml_string(&self.proxy.key()),
        )
    }

    pub fn save(&self) {
        let body = self.render();
        if let Some(p) = path()
            && std::fs::write(&p, &body).is_ok()
        {
            return;
        }

        let fallback = fallback_path();
        if let Some(dir) = fallback.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&fallback, body) {
            Ok(()) => crate::platform::util::log(&format!(
                "config saved to fallback {}",
                fallback.display()
            )),
            Err(e) => crate::platform::util::log(&format!("config save failed: {e}")),
        }
    }
}

fn lines_of(text: &str) -> impl Iterator<Item = &str> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('['))
}

fn unquote(raw: &str) -> Option<String> {
    if let Some(rest) = raw.strip_prefix('\'') {
        let end = rest.find('\'')?;
        return Some(rest[..end].to_owned());
    }
    if let Some(rest) = raw.strip_prefix('"') {
        let mut out = String::with_capacity(rest.len());
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(out),
                '\\' => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),

                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                },
                _ => out.push(c),
            }
        }
        return Some(out);
    }

    let end = raw.find('#').unwrap_or(raw.len());
    let bare = raw[..end].trim();
    (!bare.is_empty()).then(|| bare.to_owned())
}

fn toml_string(value: &str) -> String {
    if !value.contains('\'') && !value.contains('\n') {
        format!("'{value}'")
    } else {
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        format!("\"{escaped}\"")
    }
}

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

    fn sample() -> Config {
        Config {
            save_dir: PathBuf::from(r"C:\Users\me\Pictures\Shots"),
            auto_save: false,
            theme: crate::theme::Preference::Dark,
            lang: crate::config::i18n::Preference::En,
            auto_update: crate::update::AutoUpdate::Silent,
            proxy: crate::config::proxy::Proxy::parse("127.0.0.1:7890"),
        }
    }

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
    fn generated_toml_reads_back() {
        let cfg = sample();
        let text = cfg.render();
        assert!(
            text.contains(r"save_dir = 'C:\Users\me\Pictures\Shots'"),
            "{text}"
        );
        assert!(text.contains("auto_save = false"), "{text}");
        let back = Config::parse(&text);
        assert_eq!(back.save_dir, cfg.save_dir);
        assert!(!back.auto_save);
        assert_eq!(back.theme.key(), "dark");
        assert_eq!(back.lang.key(), "en");
        assert_eq!(back.auto_update.key(), "silent");
        assert_eq!(back.proxy.key(), "127.0.0.1:7890");
    }

    #[test]
    fn quote_in_a_path_switches_to_a_basic_string() {
        let mut cfg = sample();
        cfg.save_dir = PathBuf::from(r"C:\it's\shots");
        let text = cfg.render();
        assert!(text.contains(r#"save_dir = "C:\\it's\\shots""#), "{text}");
        assert_eq!(Config::parse(&text).save_dir, cfg.save_dir);
    }

    #[test]
    fn hand_edited_toml_is_accepted() {
        let text = "\u{feff}# RustShot config\n\
                    save_dir = \"D:\\\\shots\"   # hand-written comment\n\
                    auto_save = false\n\
                    theme = dark\n\
                    lang = en\n\
                    auto_update = silent\n\
                    proxy = ''\n";
        let cfg = Config::parse(text);
        assert_eq!(cfg.save_dir, PathBuf::from(r"D:\shots"));
        assert!(!cfg.auto_save);
        assert_eq!(cfg.theme.key(), "dark");
        assert_eq!(cfg.lang.key(), "en");
        assert_eq!(cfg.auto_update.key(), "silent");
        assert_eq!(cfg.proxy.key(), "system");
    }

    #[test]
    fn legacy_config_txt_still_parses() {
        let text = "save_dir=C:\\Shots\\RustShot\r\nauto_save=false\r\ntheme=light\r\n\
                    lang=zh\r\nauto_update=notice\r\nproxy=7890\r\n";
        let cfg = Config::parse(text);
        assert_eq!(cfg.save_dir, PathBuf::from(r"C:\Shots\RustShot"));
        assert!(!cfg.auto_save);
        assert_eq!(cfg.theme.key(), "light");
        assert_eq!(cfg.lang.key(), "zh");
        assert_eq!(cfg.auto_update.key(), "notice");
        assert_eq!(cfg.proxy.key(), "127.0.0.1:7890");
    }

    #[test]
    fn default_save_dir_is_a_parameter() {
        assert_eq!(
            expand_env(DEFAULT_SAVE_DIR_PARAM),
            crate::platform::util::default_save_dir()
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
