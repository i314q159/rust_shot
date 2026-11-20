// RustShot — 轻量级 Windows 截图 / 标注 / 贴图工具
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod capture;
mod fonts;
mod overlay;
mod shapes;
mod util;
mod win;
mod worker;

use capture::PhysRect;

fn main() {
    util::install_panic_logger();
    capture::set_dpi_aware();
    util::log(&format!(
        "start args={:?}",
        std::env::args().skip(1).collect::<Vec<_>>()
    ));

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut start = app::Start::Panel;
    let mut selftest: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--capture" | "-c" => start = app::Start::Region,
            "--full" | "-f" => start = app::Start::Full,
            "--pin" | "-p" => start = app::Start::Pin,
            "--selftest" => {
                selftest = Some(
                    args.get(i + 1)
                        .cloned()
                        .unwrap_or_else(|| "selftest.png".to_string()),
                );
                i += 1;
            }
            "--help" | "-h" => {
                println!(
                    "RustShot\n  (无参数)        启动主面板\n  --capture, -c  立即区域截图\n  --full, -f     立即全屏截图到剪贴板\n  --pin, -p      把剪贴板图片贴到屏幕\n  --selftest <png> 离屏渲染一张测试图并退出"
                );
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

    // Single instance: hand the request to the running process.
    let already = worker::instance_exists();
    if already {
        let cmd = match start {
            app::Start::Region => worker::CMD_REGION,
            app::Start::Full => worker::CMD_FULL,
            app::Start::Pin => worker::CMD_PIN,
            app::Start::Panel => worker::CMD_PANEL,
        };
        if worker::forward_to_running(cmd) {
            return;
        }
    }

    let icon = app::app_icon(64);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(app::PANEL_TITLE)
            .with_inner_size([470.0, 460.0])
            .with_min_inner_size([440.0, 420.0])
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    if let Err(e) = eframe::run_native(
        "RustShot",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, start)))),
    ) {
        util::log(&format!("eframe exited with error: {e}"));
    }
    util::log("app loop finished");
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
    let img = shapes::render(&frame, sel, &shapes);
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

fn demo_shapes(frame: &capture::Frame, sel: PhysRect) -> Vec<shapes::Shape> {
    use shapes::Shape;
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
            text: "RustShot 标注测试：文字 / 中文 / 123".to_string(),
            color: yellow,
            size: 28.0,
        },
    ]
}
