pub(crate) mod fonts;
pub(crate) mod i18n;
pub(crate) mod proxy;

use crate::config::i18n::strings;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "config.toml";

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

    pub fn save(&self) -> bool {
        let body = self.render();
        if let Some(p) = path()
            && write_atomic(&p, &body)
        {
            return true;
        }

        let fallback = fallback_path();
        if let Some(dir) = fallback.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if write_atomic(&fallback, &body) {
            crate::platform::util::log(&format!("config saved to fallback {}", fallback.display()));
            true
        } else {
            crate::platform::util::log(&format!("config save failed: {}", fallback.display()));
            false
        }
    }
}

fn write_atomic(target: &Path, body: &str) -> bool {
    let tmp = target.with_extension("toml.tmp");
    if std::fs::write(&tmp, body).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    if std::fs::rename(&tmp, target).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return false;
    }
    true
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
