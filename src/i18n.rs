//! UI language: Chinese or English.
//!
//! Windows' own UI language decides the default; `config.txt` (`lang=`) and the
//! top bar can pin one. Strings live at their call sites as
//! `tr("中文", "English")`, which keeps a translation next to the code that
//! shows it instead of in one giant table.

use std::sync::atomic::{AtomicU8, Ordering};

/// A concrete UI language.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    Zh,
    En,
}

impl Lang {
    /// `zh` / `en`, as written to the config file.
    pub fn key(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }

    /// Name shown in the language menu (always in its own language).
    pub fn label(self) -> &'static str {
        match self {
            Self::Zh => "中文",
            Self::En => "English",
        }
    }
}

/// User choice from the top bar, persisted in `config.txt`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preference {
    /// Follow the Windows UI language.
    #[default]
    System,
    Zh,
    En,
}

impl Preference {
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Zh => "zh",
            Self::En => "en",
        }
    }

    /// Parse a config value; anything unknown falls back to `system`.
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "zh" | "cn" | "chinese" => Self::Zh,
            "en" | "english" => Self::En,
            _ => Self::System,
        }
    }

    /// Menu label in the current UI language.
    pub fn label(self) -> String {
        match self {
            Self::System => tr("跟随系统", "Follow system").to_owned(),
            Self::Zh => Lang::Zh.label().to_owned(),
            Self::En => Lang::En.label().to_owned(),
        }
    }
}

static ACTIVE: AtomicU8 = AtomicU8::new(0);

pub fn set(lang: Lang) {
    ACTIVE.store(
        match lang {
            Lang::Zh => 0,
            Lang::En => 1,
        },
        Ordering::Relaxed,
    );
}

pub fn lang() -> Lang {
    if ACTIVE.load(Ordering::Relaxed) == 1 {
        Lang::En
    } else {
        Lang::Zh
    }
}

pub fn is_zh() -> bool {
    lang() == Lang::Zh
}

/// Windows' UI language, the default when nothing is pinned.
pub fn detect() -> Lang {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    // LANGID: low 10 bits are the primary language; 0x04 is Chinese.
    let primary = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
    if primary == 0x04 { Lang::Zh } else { Lang::En }
}

pub fn resolve(pref: Preference) -> Lang {
    match pref {
        Preference::System => detect(),
        Preference::Zh => Lang::Zh,
        Preference::En => Lang::En,
    }
}

/// `tr` for a template with one `{}` placeholder (`format!` needs a literal, so
/// these helpers do the substitution instead).
pub fn tr1(zh: &'static str, en: &'static str, a: impl std::fmt::Display) -> String {
    tr(zh, en).replacen("{}", &a.to_string(), 1)
}

/// Same as [`tr1`] with two `{}` placeholders.
pub fn tr2(
    zh: &'static str,
    en: &'static str,
    a: impl std::fmt::Display,
    b: impl std::fmt::Display,
) -> String {
    let once = tr(zh, en).replacen("{}", &a.to_string(), 1);
    once.replacen("{}", &b.to_string(), 1)
}

/// Pick the Chinese or English variant of a string.
pub fn tr(zh: &'static str, en: &'static str) -> &'static str {
    if is_zh() { zh } else { en }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_round_trips() {
        for pref in [Preference::System, Preference::Zh, Preference::En] {
            assert_eq!(Preference::parse(pref.key()), pref);
        }
        assert_eq!(Preference::parse("EN"), Preference::En);
        assert_eq!(Preference::parse("中文"), Preference::System);
        assert_eq!(Preference::parse(""), Preference::System);
    }

    #[test]
    fn switching_language_changes_the_picked_string() {
        let before = lang();
        set(Lang::Zh);
        assert_eq!(tr("中文", "English"), "中文");
        set(Lang::En);
        assert_eq!(tr("中文", "English"), "English");
        set(before);
    }

    #[test]
    fn detect_returns_a_real_language() {
        let lang = detect();
        assert!(matches!(lang, Lang::Zh | Lang::En));
        assert_eq!(lang.key(), if lang == Lang::Zh { "zh" } else { "en" });
    }
}
