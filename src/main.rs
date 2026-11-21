#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod capture;
mod config;
mod platform;
mod theme;
mod ui;
mod update;

use config::i18n::{strings, tr};

fn main() {
    platform::util::install_panic_logger();
    capture::set_dpi_aware();

    config::i18n::set(config::i18n::resolve(config::Config::load().lang));
    platform::util::log(&format!(
        "start args={:?}",
        std::env::args().skip(1).collect::<Vec<_>>()
    ));

    let args: Vec<String> = std::env::args().skip(1).collect();

    if let Some(pos) = args.iter().position(|a| a == "--apply-update") {
        let pid = args
            .get(pos + 1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        if let Some(source) = args.get(pos + 2) {
            let app = args
                .get(pos + 3)
                .map(std::path::PathBuf::from)
                .or_else(|| std::env::current_exe().ok())
                .unwrap_or_else(|| std::path::PathBuf::from(source));
            update::apply::run(pid, std::path::Path::new(source), &app);
        }
        return;
    }

    let mut requested: Option<app::Start> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--panel" | "--show" => requested = Some(app::Start::Panel),
            "--capture" | "-c" => requested = Some(app::Start::Region),
            "--full" | "-f" => requested = Some(app::Start::Full),
            "--tray" | "--silent" => requested = Some(app::Start::Tray),
            "--check-update" => requested = Some(app::Start::Check),
            "--help" | "-h" => {
                show_help();
                return;
            }
            _ => {}
        }
        i += 1;
    }

    let already = app::worker::instance_exists();
    let (start, forward) = resolve_launch(requested, already);
    if let Some(cmd) = forward
        && app::worker::forward_to_running(cmd)
    {
        return;
    }
    let Some(start) = start else {
        return;
    };

    let icon = ui::tray::app_icon(64);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(app::panel_title())
            .with_inner_size([470.0, 460.0])
            .with_min_inner_size([400.0, 300.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "RustShot",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, start)))),
    ) {
        platform::util::log(&format!("eframe exited with error: {e}"));
    }
    platform::util::log("app loop finished");
}

fn resolve_launch(
    requested: Option<app::Start>,
    already_running: bool,
) -> (Option<app::Start>, Option<usize>) {
    match (requested, already_running) {
        (None, true) | (Some(app::Start::Tray), true) => (None, None),
        (None, false) | (Some(app::Start::Tray), false) => (Some(app::Start::Tray), None),

        (Some(app::Start::Panel), true) => (None, Some(app::worker::CMD_PANEL)),
        (Some(app::Start::Region), true) => (None, Some(app::worker::CMD_REGION)),
        (Some(app::Start::Full), true) => (None, Some(app::worker::CMD_FULL)),
        (Some(app::Start::Check), true) => (None, Some(app::worker::CMD_CHECK_UPDATE)),
        (Some(start), false) => (Some(start), None),
    }
}

fn show_help() {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONINFORMATION, MB_OK, MessageBoxW};
    use windows::core::PCWSTR;
    let text = platform::util::wide(tr(strings::HELP_MESSAGE));
    let title = platform::util::wide("RustShot");
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
