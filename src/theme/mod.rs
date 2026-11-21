






use crate::config::i18n::strings;
use std::sync::atomic::{AtomicBool, Ordering};

use egui::Color32;

mod dark;
mod light;

pub use dark::DARK;
pub use light::LIGHT;


pub struct Palette {
    
    pub bg: Color32,
    
    pub card: Color32,
    
    pub bar: Color32,
    
    pub line: Color32,
    
    pub text: Color32,
    
    pub text_weak: Color32,
    
    pub text_faint: Color32,
    
    pub accent: Color32,
    
    pub accent_soft: Color32,
    
    pub ok: Color32,
    
    pub warn: Color32,
    
    pub hint: Color32,
    
    pub widget: Color32,
    
    pub widget_hover: Color32,
    
    pub widget_active: Color32,
    
    pub widget_stroke: Color32,
}


static DARK_ACTIVE: AtomicBool = AtomicBool::new(false);


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preference {
    
    #[default]
    System,
    Light,
    Dark,
}

impl Preference {
    
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => crate::config::i18n::tr(strings::FOLLOW_SYSTEM),
            Self::Light => crate::config::i18n::tr(strings::LIGHT),
            Self::Dark => crate::config::i18n::tr(strings::DARK),
        }
    }
}


pub fn resolve(pref: Preference) -> bool {
    match pref {
        Preference::System => system_is_dark(),
        Preference::Light => false,
        Preference::Dark => true,
    }
}


pub fn palette() -> &'static Palette {
    if DARK_ACTIVE.load(Ordering::Relaxed) {
        &DARK
    } else {
        &LIGHT
    }
}

pub fn is_dark() -> bool {
    DARK_ACTIVE.load(Ordering::Relaxed)
}

pub fn bg() -> Color32 {
    palette().bg
}

pub fn card() -> Color32 {
    palette().card
}

pub fn bar() -> Color32 {
    palette().bar
}

pub fn line() -> Color32 {
    palette().line
}

pub fn text() -> Color32 {
    palette().text
}

pub fn text_weak() -> Color32 {
    palette().text_weak
}

pub fn text_faint() -> Color32 {
    palette().text_faint
}

pub fn accent() -> Color32 {
    palette().accent
}

pub fn ok() -> Color32 {
    palette().ok
}

pub fn warn() -> Color32 {
    palette().warn
}

pub fn hint() -> Color32 {
    palette().hint
}







pub fn system_is_dark() -> bool {
    use windows::Win32::Foundation::WIN32_ERROR;
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
    use windows::core::PCWSTR;

    let subkey = crate::platform::util::wide(
        r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
    );
    let name = crate::platform::util::wide("AppsUseLightTheme");
    let mut value = 1u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(name.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut std::ffi::c_void),
            Some(&mut size),
        )
    };
    status == WIN32_ERROR(0) && value == 0
}







pub fn apply(ctx: &egui::Context, dark: bool) {
    DARK_ACTIVE.store(dark, Ordering::Relaxed);
    ctx.set_visuals_of(egui::Theme::Light, visuals(&LIGHT, false));
    ctx.set_visuals_of(egui::Theme::Dark, visuals(&DARK, true));
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
}

fn visuals(p: &Palette, dark: bool) -> egui::Visuals {
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.extreme_bg_color = p.card;
    v.faint_bg_color = p.hint;
    v.override_text_color = Some(p.text);
    v.selection.bg_fill = p.accent_soft;
    
    
    v.selection.stroke = egui::Stroke::new(1.0, p.text);

    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, p.line);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, p.text_weak);

    
    
    v.widgets.inactive.weak_bg_fill = p.widget;
    v.widgets.inactive.bg_fill = p.widget;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, p.line);
    v.widgets.hovered.weak_bg_fill = p.widget_hover;
    v.widgets.hovered.bg_fill = p.widget_hover;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, p.widget_stroke);
    v.widgets.active.weak_bg_fill = p.widget_active;
    v.widgets.active.bg_fill = p.widget_active;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, p.accent);
    
    v.widgets.open.weak_bg_fill = p.widget_hover;
    v.widgets.open.bg_fill = p.widget_hover;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0, p.line);

    if dark {
        
        
        v.window_stroke = egui::Stroke::new(1.0, p.line);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    
    fn luminance(c: Color32) -> f64 {
        let channel = |v: u8| {
            let s = v as f64 / 255.0;
            if s <= 0.039_28 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(c.r()) + 0.7152 * channel(c.g()) + 0.0722 * channel(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (la, lb) = (luminance(a), luminance(b));
        let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn both_themes_keep_text_readable() {
        for (name, p) in [("light", &LIGHT), ("dark", &DARK)] {
            assert!(
                contrast(p.text, p.bg) >= 7.0,
                "{name}: body text {:.1}:1",
                contrast(p.text, p.bg)
            );
            assert!(contrast(p.text, p.card) >= 7.0, "{name}: text on card");
            assert!(
                contrast(p.text_weak, p.card) >= 4.5,
                "{name}: secondary text {:.1}:1",
                contrast(p.text_weak, p.card)
            );
            assert!(
                contrast(p.text_faint, p.card) >= 3.0,
                "{name}: captions {:.1}:1",
                contrast(p.text_faint, p.card)
            );
            assert!(
                contrast(p.accent, p.card) >= 3.0,
                "{name}: accent {:.1}:1",
                contrast(p.accent, p.card)
            );
            assert!(
                contrast(p.ok, p.card) >= 3.5,
                "{name}: ok status {:.1}:1",
                contrast(p.ok, p.card)
            );
            assert!(
                contrast(p.warn, p.card) >= 3.5,
                "{name}: warn status {:.1}:1",
                contrast(p.warn, p.card)
            );
        }
    }

    #[test]
    fn theme_preference_round_trips() {
        assert_eq!(Preference::parse("light"), Preference::Light);
        assert_eq!(Preference::parse(" Dark "), Preference::Dark);
        assert_eq!(Preference::parse("SYSTEM"), Preference::System);
        assert_eq!(Preference::parse("nonsense"), Preference::System);
        assert_eq!(Preference::parse(""), Preference::System);
        for pref in [Preference::System, Preference::Light, Preference::Dark] {
            assert_eq!(Preference::parse(pref.key()), pref);
            assert!(!pref.label().is_empty());
        }
        assert_eq!(Preference::default(), Preference::System);
    }

    #[test]
    fn themes_are_not_the_same_palette() {
        assert_ne!(LIGHT.bg, DARK.bg);
        assert_ne!(LIGHT.card, DARK.card);
        
        assert!(luminance(DARK.bg) < luminance(LIGHT.bg));
        assert!(luminance(DARK.text) > luminance(LIGHT.text));
    }
}
