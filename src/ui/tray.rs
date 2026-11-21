//! Tray icon, its menu and the embedded window icon.

use crate::config::i18n::strings;
use crate::config::i18n::tr;

const ICON_32: &[u8] = include_bytes!("../../assets/icon/rustshot-32.png");
const ICON_64: &[u8] = include_bytes!("../../assets/icon/rustshot-64.png");
const ICON_256: &[u8] = include_bytes!("../../assets/icon/rustshot-256.png");

pub(crate) struct MenuHandles {
    pub(crate) region: muda::MenuItem,
    pub(crate) full: muda::MenuItem,
    pub(crate) panel: muda::MenuItem,
    pub(crate) quit: muda::MenuItem,
}

pub(crate) fn app_icon(size: u32) -> egui::IconData {
    let bytes = match size {
        0..=32 => ICON_32,
        33..=64 => ICON_64,
        _ => ICON_256,
    };
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .expect("embedded icon is a valid PNG")
        .to_rgba8();
    egui::IconData {
        rgba: img.as_raw().clone(),
        width: img.width(),
        height: img.height(),
    }
}

pub(crate) fn build_tray() -> (Option<tray_icon::TrayIcon>, Option<MenuHandles>) {
    let menu = muda::Menu::new();
    let region = muda::MenuItem::new(tr(strings::REGION_CAPTURE), true, None);
    let full = muda::MenuItem::new(tr(strings::FULL_SCREEN), true, None);
    let panel = muda::MenuItem::new(tr(strings::SHOW_PANEL), true, None);
    let quit = muda::MenuItem::new(tr(strings::QUIT), true, None);
    for item in [&region, &full, &panel, &quit] {
        let _ = menu.append(item);
    }
    let icon = app_icon(32);
    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip(tr(strings::RUSTSHOT))
        .with_icon(
            match tray_icon::Icon::from_rgba(icon.rgba, icon.width, icon.height) {
                Ok(i) => i,
                Err(_) => return (None, None),
            },
        )
        .with_menu(Box::new(menu))
        // Left click starts a capture; the menu belongs to the right button.
        .with_menu_on_left_click(false)
        .build()
        .ok();
    (
        tray,
        Some(MenuHandles {
            region,
            full,
            panel,
            quit,
        }),
    )
}
