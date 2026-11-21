// RustShot — 轻量级 Windows 截图 / 标注工具
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod capture;
mod config;
mod platform;
mod theme;
mod ui;
mod update;

use capture::PhysRect;
use config::i18n::{strings, tr};

fn main() {
    platform::util::install_panic_logger();
    capture::set_dpi_aware();
    // The help box and the window title need the language before the app exists.
    config::i18n::set(config::i18n::resolve(config::Config::load().lang));
    platform::util::log(&format!(
        "start args={:?}",
        std::env::args().skip(1).collect::<Vec<_>>()
    ));

    let args: Vec<String> = std::env::args().skip(1).collect();

    // Hidden helper: `--apply-update <pid> <file>` installs a verified download
    // after the app has exited and starts it again. Runs before any UI setup.
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
    // `None` = no flag given: a plain launch parks itself in the tray.
    let mut requested: Option<app::Start> = None;
    let mut selftest: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--panel" | "--show" => requested = Some(app::Start::Panel),
            "--capture" | "-c" => requested = Some(app::Start::Region),
            "--full" | "-f" => requested = Some(app::Start::Full),
            "--tray" | "--silent" => requested = Some(app::Start::Tray),
            "--check-update" => requested = Some(app::Start::Check),
            "--selftest" => {
                selftest = Some(
                    args.get(i + 1)
                        .cloned()
                        .unwrap_or_else(|| "selftest.png".to_string()),
                );
                i += 1;
            }
            "--help" | "-h" => {
                show_help();
                return;
            }
            _ => {}
        }
        i += 1;
    }

    if let Some(out) = selftest {
        selftest_run(&out);
        return;
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
            // 允许压到比较小：面板主体是可滚动的，底栏固定在底部。
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

/// What this process should do, given its flags and the single-instance state.
///
/// `None` as the start mode means "exit quietly".
fn resolve_launch(
    requested: Option<app::Start>,
    already_running: bool,
) -> (Option<app::Start>, Option<usize>) {
    match (requested, already_running) {
        // A plain launch (double click, shortcut, autostart) is a tray app: the
        // resident copy is already there, so stay quiet instead of stealing focus.
        (None, true) | (Some(app::Start::Tray), true) => (None, None),
        (None, false) | (Some(app::Start::Tray), false) => (Some(app::Start::Tray), None),
        // Explicit window request: hand it to the running instance.
        (Some(app::Start::Panel), true) => (None, Some(app::worker::CMD_PANEL)),
        (Some(app::Start::Region), true) => (None, Some(app::worker::CMD_REGION)),
        (Some(app::Start::Full), true) => (None, Some(app::worker::CMD_FULL)),
        (Some(app::Start::Check), true) => (None, Some(app::worker::CMD_CHECK_UPDATE)),
        (Some(start), false) => (Some(start), None),
    }
}

/// `--help` must be visible in a GUI-subsystem build, where stdout goes nowhere.
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

/// Render a sample annotated image without opening any window; useful for
/// verifying the export pipeline (`rust_shot.exe --selftest out.png`).
fn selftest_run(out: &str) {
    let mon = capture::monitor_at_cursor();
    let Some(frame) = capture::capture(mon) else {
        eprintln!("capture failed");
        std::process::exit(1);
    };
    let w = (frame.w as i32 - 120).clamp(200, 1000);
    let h = (frame.h as i32 - 160).clamp(150, 600);
    let sel = PhysRect::new(60, 60, w, h);
    let shapes = demo_shapes(&frame, sel);
    let img = capture::shapes::render(&frame, sel, &shapes);
    if let Some(parent) = std::path::Path::new(out).parent()
        && !parent.as_os_str().is_empty()
    {
        let _ = std::fs::create_dir_all(parent);
    }
    match image::save_buffer(
        out,
        img.as_raw(),
        img.width(),
        img.height(),
        image::ColorType::Rgba8,
    ) {
        Ok(()) => println!("saved {} ({}x{})", out, img.width(), img.height()),
        Err(e) => {
            eprintln!("save failed: {e}");
            std::process::exit(1);
        }
    }
}

fn demo_shapes(frame: &capture::Frame, sel: PhysRect) -> Vec<capture::shapes::Shape> {
    use capture::shapes::Shape;
    let red = egui::Color32::from_rgb(232, 69, 60);
    let yellow = egui::Color32::from_rgb(247, 201, 72);
    let blue = egui::Color32::from_rgb(42, 127, 255);
    let green = egui::Color32::from_rgb(47, 191, 113);

    let mut pen_pts = Vec::new();
    for i in 0..60 {
        let t = i as f32 / 59.0;
        pen_pts.push((
            sel.x as f32 + 60.0 + t * 320.0,
            sel.y as f32 + 220.0 - (t * std::f32::consts::TAU).sin() * 55.0,
        ));
    }

    // mosaic over a 180x70 region
    let mut blocks = Vec::new();
    let cell = 9.0f32;
    let mx = sel.x as f32 + 60.0;
    let my = sel.y as f32 + 300.0;
    let mut gy = 0;
    while gy < 8 {
        let mut gx = 0;
        while gx < 20 {
            let px = mx + gx as f32 * cell;
            let py = my + gy as f32 * cell;
            let c = frame.pixel(px as i32 + 4, py as i32 + 4);
            blocks.push((px, py, c));
            gx += 1;
        }
        gy += 1;
    }

    vec![
        Shape::Rect {
            a: (sel.x as f32 + 30.0, sel.y as f32 + 30.0),
            b: (sel.x as f32 + 400.0, sel.y as f32 + 200.0),
            color: red,
            w: 4.0,
        },
        Shape::Arrow {
            a: (sel.x as f32 + 430.0, sel.y as f32 + 180.0),
            b: (sel.x as f32 + 600.0, sel.y as f32 + 60.0),
            color: blue,
            w: 5.0,
        },
        Shape::Pen {
            pts: pen_pts,
            color: green,
            w: 4.0,
        },
        Shape::Mosaic { cell, blocks },
        Shape::Number {
            center: (sel.x as f32 + 560.0, sel.y as f32 + 260.0),
            n: 1,
            color: yellow,
            r: 18.0,
        },
        Shape::Text {
            pos: (sel.x as f32 + 60.0, sel.y as f32 + 420.0),
            text: tr(strings::SELFTEST).to_string(),
            color: yellow,
            size: 28.0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_launch_parks_in_the_tray() {
        assert_eq!(resolve_launch(None, false), (Some(app::Start::Tray), None));
        // Already resident: a second double click must not steal focus.
        assert_eq!(resolve_launch(None, true), (None, None));
        assert_eq!(resolve_launch(Some(app::Start::Tray), true), (None, None));
        assert_eq!(
            resolve_launch(Some(app::Start::Tray), false),
            (Some(app::Start::Tray), None)
        );
    }

    #[test]
    fn explicit_window_requests_reach_the_running_instance() {
        assert_eq!(
            resolve_launch(Some(app::Start::Panel), true),
            (None, Some(app::worker::CMD_PANEL))
        );
        assert_eq!(
            resolve_launch(Some(app::Start::Panel), false),
            (Some(app::Start::Panel), None)
        );
        assert_eq!(
            resolve_launch(Some(app::Start::Region), true),
            (None, Some(app::worker::CMD_REGION))
        );
        assert_eq!(
            resolve_launch(Some(app::Start::Full), true),
            (None, Some(app::worker::CMD_FULL))
        );
    }
}
