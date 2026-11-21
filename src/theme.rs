//! One light palette shared by the control panel and the capture overlay.
//!
//! Kept in one place so the panel card, the toolbar and the size tag never drift
//! apart: a screenshot tool is used on top of arbitrary content, and light chrome
//! keeps the frozen frame itself the brightest thing on screen.

use egui::Color32;

/// Window background (slightly off-white so white cards stand out).
pub const BG: Color32 = Color32::from_rgb(0xF7, 0xF8, 0xFA);
/// Cards, toolbars, popups.
pub const CARD: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
/// Menu bar strip.
pub const BAR: Color32 = Color32::from_rgb(0xEF, 0xF1, 0xF4);
/// Hairline borders, separators.
pub const LINE: Color32 = Color32::from_rgb(0xDC, 0xE0, 0xE6);
/// Body text.
pub const TEXT: Color32 = Color32::from_rgb(0x1F, 0x23, 0x29);
/// Secondary text.
pub const TEXT_WEAK: Color32 = Color32::from_rgb(0x5C, 0x63, 0x6E);
/// Labels, captions, hints.
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x8A, 0x91, 0x9C);
/// Selection blue, same as the capture outline.
pub const ACCENT: Color32 = Color32::from_rgb(0x2A, 0x7F, 0xFF);
/// Tinted background for the active tool / selected row.
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xE4, 0xEF, 0xFF);
/// Status text: done.
pub const OK: Color32 = Color32::from_rgb(0x1E, 0x8E, 0x5A);
/// Status text: problem.
pub const WARN: Color32 = Color32::from_rgb(0xC0, 0x4A, 0x32);

/// Install the light theme on a context (panel and overlay share it).
pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::light();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = CARD;
    v.faint_bg_color = Color32::from_rgb(0xF1, 0xF3, 0xF6);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT_SOFT;
    // egui uses `selection.stroke.color` as the *text* colour of a selected
    // selectable label / selected text, so it must stay dark on the pale tint.
    v.selection.stroke = egui::Stroke::new(1.0, TEXT);

    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, LINE);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_WEAK);

    // Buttons sit on an off-white background, so they need a visible border
    // instead of the flat grey of egui's default light widgets.
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0xF2, 0xF4, 0xF7);
    v.widgets.inactive.bg_fill = Color32::from_rgb(0xF2, 0xF4, 0xF7);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, LINE);
    let hover = Color32::from_rgb(0xE6, 0xEA, 0xF1);
    v.widgets.hovered.weak_bg_fill = hover;
    v.widgets.hovered.bg_fill = hover;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(0xC2, 0xC9, 0xD4));
    let active = Color32::from_rgb(0xD9, 0xDF, 0xE8);
    v.widgets.active.weak_bg_fill = active;
    v.widgets.active.bg_fill = active;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0, ACCENT);

    ctx.set_visuals(v);
}
