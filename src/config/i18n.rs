pub mod strings;

use std::sync::atomic::{AtomicU8, Ordering};
use strings::Text;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    Zh,
    En,
}

impl Lang {
    pub fn key(self) -> &'static str {
        match self {
            Self::Zh => "zh",
            Self::En => "en",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Zh => strings::LANGUAGE_ZH,
            Self::En => strings::LANGUAGE_EN,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preference {
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

    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "zh" | "cn" | "chinese" => Self::Zh,
            "en" | "english" => Self::En,
            _ => Self::System,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::System => tr(strings::FOLLOW_SYSTEM).to_owned(),
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

pub fn detect() -> Lang {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;

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

pub fn tr(text: Text) -> &'static str {
    if is_zh() { text.zh } else { text.en }
}

pub fn tr1(text: Text, a: impl std::fmt::Display) -> String {
    tr(text).replacen("{}", &a.to_string(), 1)
}

pub fn tr2(text: Text, a: impl std::fmt::Display, b: impl std::fmt::Display) -> String {
    let once = tr(text).replacen("{}", &a.to_string(), 1);
    once.replacen("{}", &b.to_string(), 1)
}

pub fn tr3(
    text: Text,
    a: impl std::fmt::Display,
    b: impl std::fmt::Display,
    c: impl std::fmt::Display,
) -> String {
    let once = tr2(text, a, b);
    once.replacen("{}", &c.to_string(), 1)
}
