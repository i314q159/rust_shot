//! Dark palette for the control panel and capture overlay.

use egui::Color32;

use super::Palette;

/// One complete set of dark chrome colours.
pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x1A, 0x1C, 0x20),
    card: Color32::from_rgb(0x24, 0x27, 0x2C),
    bar: Color32::from_rgb(0x15, 0x17, 0x1B),
    line: Color32::from_rgb(0x34, 0x38, 0x3F),
    text: Color32::from_rgb(0xE7, 0xE9, 0xED),
    text_weak: Color32::from_rgb(0xA9, 0xB1, 0xBC),
    text_faint: Color32::from_rgb(0x85, 0x8E, 0x99),
    accent: Color32::from_rgb(0x55, 0x98, 0xFF),
    accent_soft: Color32::from_rgb(0x1E, 0x33, 0x52),
    ok: Color32::from_rgb(0x4F, 0xC2, 0x8C),
    warn: Color32::from_rgb(0xE8, 0x83, 0x6A),
    hint: Color32::from_rgb(0x20, 0x23, 0x29),
    widget: Color32::from_rgb(0x2A, 0x2E, 0x34),
    widget_hover: Color32::from_rgb(0x34, 0x3A, 0x42),
    widget_active: Color32::from_rgb(0x3C, 0x43, 0x4C),
    widget_stroke: Color32::from_rgb(0x4A, 0x52, 0x60),
};
