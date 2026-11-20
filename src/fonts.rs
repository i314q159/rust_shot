//! System CJK font loading, shared by the egui UI and the exporter.

use std::sync::{Arc, OnceLock};

pub struct LoadedFont {
    pub bytes: Arc<Vec<u8>>,
    pub index: u32,
    pub label: String,
}

static FONT: OnceLock<Option<LoadedFont>> = OnceLock::new();
static RASTER: OnceLock<Option<fontdue::Font>> = OnceLock::new();

const CANDIDATES: &[(&str, u32, &str)] = &[
    ("C:\\Windows\\Fonts\\msyh.ttc", 0, "Microsoft YaHei"),
    ("C:\\Windows\\Fonts\\msyh.ttf", 0, "Microsoft YaHei"),
    ("C:\\Windows\\Fonts\\simhei.ttf", 0, "SimHei"),
    ("C:\\Windows\\Fonts\\simsun.ttc", 0, "SimSun"),
    ("C:\\Windows\\Fonts\\segoeui.ttf", 0, "Segoe UI"),
];

fn probe(path: &str, index: u32) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 1024 {
        return None;
    }
    // Make sure the file really is a font fontdue can use.
    let settings = fontdue::FontSettings {
        collection_index: index,
        ..Default::default()
    };
    fontdue::Font::from_bytes(bytes.as_slice(), settings).ok()?;
    Some(bytes)
}

fn load() -> Option<LoadedFont> {
    for (path, index, label) in CANDIDATES {
        if let Some(bytes) = probe(path, *index) {
            return Some(LoadedFont {
                bytes: Arc::new(bytes),
                index: *index,
                label: (*label).to_string(),
            });
        }
    }
    None
}

/// Bytes + face index of the UI/annotation font.
pub fn cjk() -> Option<&'static LoadedFont> {
    FONT.get_or_init(load).as_ref()
}

/// Font used by the raster exporter.
pub fn raster() -> Option<&'static fontdue::Font> {
    RASTER
        .get_or_init(|| {
            let f = cjk()?;
            let settings = fontdue::FontSettings {
                collection_index: f.index,
                scale: 40.0,
                ..Default::default()
            };
            fontdue::Font::from_bytes(f.bytes.as_slice(), settings).ok()
        })
        .as_ref()
}

/// Install the CJK font into egui (as a fallback for the default families).
pub fn install_egui_font(ctx: &egui::Context) {
    let Some(font) = cjk() else { return };
    let mut defs = egui::FontDefinitions::default();
    defs.font_data.insert(
        "cjk".to_owned(),
        Arc::new(egui::FontData::from_blob(font.bytes.clone(), font.index)),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        defs.families
            .entry(family)
            .or_default()
            .push("cjk".to_owned());
    }
    ctx.set_fonts(defs);
}
