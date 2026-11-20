//! eframe application: control panel, overlay orchestration, clipboard / file plumbing.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use crate::capture::{self, PhysRect};
use crate::overlay::{self, Export, Outcome, Session, Tool};
use crate::worker::{Worker, WorkerEvent};

/// Window title of the control panel (also used to find the HWND).
pub const PANEL_TITLE: &str = "RustShot 截图工具";

static UI_CTX: OnceLock<egui::Context> = OnceLock::new();

/// Called from the worker thread to wake the UI.
pub fn request_repaint() {
    if let Some(ctx) = UI_CTX.get() {
        ctx.request_repaint();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Start {
    Panel,
    Region,
    Full,
}

/// Result of the (worker thread) save dialog.
enum SaveResult {
    Saved(PathBuf),
    Failed(String),
    Cancelled,
}

struct MenuHandles {
    region: muda::MenuItem,
    full: muda::MenuItem,
    panel: muda::MenuItem,
    quit: muda::MenuItem,
}

pub struct App {
    worker: Worker,
    session: Option<Session>,
    save_dir: PathBuf,
    auto_save: bool,
    status: String,
    tray: Option<tray_icon::TrayIcon>,
    menu: Option<MenuHandles>,
    start: Option<Start>,
    /// True only when the user really asked to quit (tray menu / exit button).
    quitting: bool,
    /// Set while the panel is being restored so the overlay can be created.
    waiting_panel: bool,
    /// Frames still to wait before the restored panel may host a viewport.
    restore_frames: u8,
    /// Bumped per capture so every overlay gets a fresh viewport id.
    serial: u64,
    /// Put the panel back into the tray once the capture is finished.
    remiminimize: bool,
    /// RequestPaintWhileHidden only needs to be sent once per capture.
    paint_hidden_sent: bool,
    /// Set while the save dialog runs on its own thread.
    save_rx: Option<std::sync::mpsc::Receiver<SaveResult>>,
    /// Set while the "pick folder" dialog runs on its own thread.
    dir_rx: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Start) -> Self {
        let _ = UI_CTX.set(cc.egui_ctx.clone());
        crate::fonts::install_egui_font(&cc.egui_ctx);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(22, 23, 26);
        visuals.override_text_color = Some(egui::Color32::from_rgb(226, 229, 234));
        cc.egui_ctx.set_visuals(visuals);
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(10.0, 6.0);
        });

        let worker = Worker::spawn();
        let (tray, menu) = build_tray();
        let cfg = crate::config::Config::load();
        crate::util::log(&format!("app created, tray={}", tray.is_some()));

        Self {
            worker,
            session: None,
            save_dir: cfg.save_dir,
            auto_save: cfg.auto_save,
            status: String::new(),
            tray,
            menu,
            start: Some(start),
            quitting: false,
            waiting_panel: false,
            restore_frames: 0,
            serial: 0,
            remiminimize: false,
            paint_hidden_sent: false,
            save_rx: None,
            dir_rx: None,
        }
    }

    /// Folder picker on its own thread: `pick_folder` also owns a modal loop.
    fn pick_dir(&mut self) {
        if self.dir_rx.is_some() {
            self.status = "目录选择框已经打开了".to_owned();
            return;
        }
        let start = self.save_dir.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.dir_rx = Some(rx);
        self.status = "正在选择保存目录…".to_owned();
        std::thread::Builder::new()
            .name("rustshot-pick".into())
            .spawn(move || {
                let picked = rfd::FileDialog::new().set_directory(&start).pick_folder();
                let _ = tx.send(picked);
                crate::app::request_repaint();
            })
            .ok();
    }

    fn poll_dir(&mut self) {
        let Some(rx) = self.dir_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(Some(dir)) => {
                self.save_dir = dir;
                self.persist();
                self.status = format!("保存目录：{}", self.save_dir.display());
            }
            Ok(None) => self.status = "已取消选择目录".to_owned(),
            Err(std::sync::mpsc::TryRecvError::Empty) => self.dir_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status = "目录选择失败".to_owned();
            }
        }
    }

    fn poll_save(&mut self) {
        let Some(rx) = self.save_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(SaveResult::Saved(path)) => {
                crate::util::log(&format!("saved {}", path.display()));
                self.status = format!("已保存 {}", path.display());
            }
            Ok(SaveResult::Failed(e)) => self.status = format!("保存失败：{e}"),
            Ok(SaveResult::Cancelled) => self.status = "已取消保存".to_owned(),
            Err(std::sync::mpsc::TryRecvError::Empty) => self.save_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status = "保存失败：对话框线程退出".to_owned();
            }
        }
    }

    fn pump_worker(&mut self, ctx: &egui::Context) {
        let mut events = Vec::new();
        while let Ok(ev) = self.worker.rx.try_recv() {
            events.push(ev);
        }
        for ev in events {
            match ev {
                WorkerEvent::CaptureRegion => self.begin_capture(ctx, Tool::Select),
                WorkerEvent::CaptureFull => self.capture_full(),
                WorkerEvent::ShowPanel => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                WorkerEvent::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn pump_tray(&mut self, ctx: &egui::Context) {
        let mut action: Option<u8> = None;
        if let Some(m) = &self.menu {
            while let Ok(ev) = muda::MenuEvent::receiver().try_recv() {
                if ev.id == m.region.id() {
                    action = Some(0);
                } else if ev.id == m.full.id() {
                    action = Some(1);
                } else if ev.id == m.panel.id() {
                    action = Some(3);
                } else if ev.id == m.quit.id() {
                    action = Some(4);
                }
            }
        }
        while let Ok(ev) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                button_state: tray_icon::MouseButtonState::Up,
                ..
            } = ev
            {
                action = Some(0);
            }
        }
        match action {
            Some(0) => self.begin_capture(ctx, Tool::Select),
            Some(1) => self.capture_full(),
            Some(2) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            Some(3) => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }

    fn begin_capture(&mut self, ctx: &egui::Context, tool: Tool) {
        let monitor = capture::monitor_at_cursor();
        let hover = capture::window_rect_at_cursor();
        crate::util::log(&format!(
            "begin_capture tool={tool:?} monitor={monitor:?} hover={hover:?}"
        ));
        match capture::capture(monitor) {
            Some(frame) => {
                self.serial = self.serial.wrapping_add(1);
                self.paint_hidden_sent = false;
                self.session = Some(Session::new(
                    Arc::new(frame),
                    monitor,
                    overlay::overlay_title(self.serial),
                    tool,
                    hover,
                ));
                self.status.clear();
                // eframe cannot create child viewports from a hidden window, so a
                // capture started from the tray restores the panel for one frame.
                if !crate::win::is_ready(PANEL_TITLE) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    self.waiting_panel = true;
                    self.restore_frames = 2;
                    self.remiminimize = true;
                }
            }
            None => self.status = "截屏失败：无法读取屏幕内容".to_owned(),
        }
    }

    fn capture_full(&mut self) {
        let monitor = capture::monitor_at_cursor();
        match capture::capture(monitor) {
            Some(frame) => {
                let rect = PhysRect::new(0, 0, frame.w as i32, frame.h as i32);
                let img = crate::shapes::render(&frame, rect, &[]);
                self.deliver(Outcome {
                    action: Export::Copy,
                    image: img,
                });
            }
            None => self.status = "截屏失败：无法读取屏幕内容".to_owned(),
        }
    }

    fn run_overlay(&mut self, ctx: &egui::Context) {
        if self.session.is_none() {
            return;
        }
        // A minimized/hidden window cannot host child viewports: wait until the
        // panel is really visible again (plus a couple of frames) before creating
        // the overlay window, otherwise the backend aborts the process.
        if self.waiting_panel {
            if self.restore_frames > 0 {
                self.restore_frames -= 1;
                ctx.request_repaint_after(std::time::Duration::from_millis(25));
                return;
            }
            if !crate::win::is_ready(PANEL_TITLE) {
                ctx.request_repaint_after(std::time::Duration::from_millis(25));
                return;
            }
            self.waiting_panel = false;
            crate::util::log("panel restored, creating overlay");
        }
        let mut failed = false;
        let (outcome, cancelled) = {
            let Some(sess) = self.session.as_mut() else {
                return;
            };
            let mon = sess.monitor;
            // A fresh id per capture: reusing one can race with the previous
            // overlay window being torn down.
            let id = egui::ViewportId::from_hash_of(("rustshot_overlay", self.serial));
            let title = overlay::overlay_title(self.serial);
            let builder = egui::ViewportBuilder::default()
                .with_title(title)
                .with_decorations(false)
                .with_resizable(false)
                .with_always_on_top()
                .with_taskbar(false)
                .with_active(true)
                .with_inner_size(egui::vec2(mon.w as f32, mon.h as f32))
                .with_position(egui::pos2(mon.x as f32, mon.y as f32));
            let shown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                ctx.show_viewport_immediate(id, builder, |ui, _class| {
                    sess.ui(ui);
                });
            }));
            if shown.is_err() {
                failed = true;
                (None, false)
            } else {
                (sess.outcome.take(), sess.cancelled)
            }
        };
        if failed {
            crate::util::log("overlay viewport creation failed, capture aborted");
            self.session = None;
            self.status = "创建截图覆盖层失败，请重试".to_owned();
        } else if let Some(o) = outcome {
            self.session = None;
            self.deliver(o);
        } else if cancelled {
            self.session = None;
            self.status = "已取消".to_owned();
        }
        if self.session.is_none() && self.remiminimize {
            // Capture started from the tray: go back to the tray.
            self.remiminimize = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn deliver(&mut self, outcome: Outcome) {
        let img = outcome.image;
        match outcome.action {
            Export::Copy => {
                let mut msg = match copy_to_clipboard(&img) {
                    Ok(()) => "已复制到剪贴板".to_owned(),
                    Err(e) => format!("复制失败：{e}"),
                };
                if self.auto_save {
                    match self.save_auto(&img) {
                        Ok(p) => msg.push_str(&format!("，已保存 {}", p.display())),
                        Err(e) => msg.push_str(&format!("，自动保存失败：{e}")),
                    }
                }
                self.status = msg;
            }
            Export::Save => {
                if self.save_rx.is_some() {
                    self.status = "保存对话框已经打开了".to_owned();
                    return;
                }
                let dir = self.save_dir.clone();
                let stem = format!("RustShot_{}", crate::util::timestamp());
                // `set_file_name` wants a bare file name, not a full path.
                let file_name = crate::config::unique_png(&dir, &stem)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "RustShot.png".to_owned());
                let (tx, rx) = std::sync::mpsc::channel();
                self.save_rx = Some(rx);
                self.status = "正在选择保存位置…".to_owned();
                // The dialog owns a nested message loop and a COM apartment:
                // running it inside our paint pass froze the window, and it also
                // ended up hidden behind the always-on-top overlay.
                std::thread::Builder::new()
                    .name("rustshot-save".into())
                    .spawn(move || {
                        let picked = rfd::FileDialog::new()
                            .set_directory(&dir)
                            .set_file_name(file_name)
                            .add_filter("PNG 图片", &["png"])
                            .save_file();
                        let result = match picked {
                            Some(path) => match save_png(&img, &path) {
                                Ok(()) => SaveResult::Saved(path),
                                Err(e) => SaveResult::Failed(e),
                            },
                            None => SaveResult::Cancelled,
                        };
                        let _ = tx.send(result);
                        crate::app::request_repaint();
                    })
                    .ok();
            }
        }
    }

    fn persist(&self) {
        crate::config::Config {
            save_dir: self.save_dir.clone(),
            auto_save: self.auto_save,
        }
        .save();
    }

    fn save_auto(&self, img: &image::RgbaImage) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.save_dir).map_err(|e| e.to_string())?;
        let stem = format!("RustShot_{}", crate::util::timestamp());
        let path = crate::config::unique_png(&self.save_dir, &stem);
        save_png(img, &path)?;
        Ok(path)
    }

    fn panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(22, 23, 26))
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("RustShot").size(24.0).strong());
                    ui.label(
                        egui::RichText::new("截图 · 标注")
                            .size(12.0)
                            .color(egui::Color32::from_rgb(130, 137, 148)),
                    );
                });
                ui.add_space(10.0);

                let bw = (ui.available_width() - 16.0) / 2.0;
                ui.horizontal(|ui| {
                    if big_button(ui, "区域截图", "F1", bw).clicked() {
                        let ctx = ui.ctx().clone();
                        self.begin_capture(&ctx, Tool::Select);
                    }
                    if big_button(ui, "全屏截图", "F2", bw).clicked() {
                        self.capture_full();
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(egui::RichText::new("保存目录").size(12.5).color(egui::Color32::from_rgb(150, 157, 168)));
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(self.save_dir.display().to_string())
                                .size(12.0)
                                .monospace(),
                        )
                        .truncate(),
                    );
                    if ui.small_button("更改").clicked() {
                        self.pick_dir();
                    }
                    if ui.small_button("打开").clicked() {
                        let _ = std::fs::create_dir_all(&self.save_dir);
                        let _ = std::process::Command::new("explorer")
                            .arg(&self.save_dir)
                            .spawn();
                    }
                });
                if ui
                    .checkbox(&mut self.auto_save, "复制到剪贴板时同时保存 PNG")
                    .changed()
                {
                    self.persist();
                }

                ui.add_space(8.0);
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(30, 32, 36))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(
                                "截图后：拖选区域 · 单击选中窗口 · Enter 复制 · Ctrl+S 另存为",
                            )
                            .size(11.5)
                            .color(egui::Color32::from_rgb(160, 166, 176)),
                        );
                        ui.label(
                            egui::RichText::new(
                                "标注：矩形 / 箭头 / 画笔 / 文字 / 马赛克 / 序号，Ctrl+Z 撤销，Ctrl+A 全选",
                            )
                            .size(11.5)
                            .color(egui::Color32::from_rgb(160, 166, 176)),
                        );
                        ui.label(
                            egui::RichText::new("全局热键：F1 区域截图 · F2 全屏 · Ctrl+Alt+Q 退出")
                                .size(11.5)
                                .color(egui::Color32::from_rgb(160, 166, 176)),
                        );
                    });

                ui.add_space(8.0);
                if !self.status.is_empty() {
                    ui.label(
                        egui::RichText::new(&self.status)
                            .size(12.0)
                            .color(egui::Color32::from_rgb(120, 200, 150)),
                    );
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        if self.tray.is_none() {
                            ui.label(
                                egui::RichText::new("托盘不可用")
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(220, 140, 120)),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!("字体：{}", crate::fonts::cjk().map(|f| f.label.clone()).unwrap_or_else(|| "系统默认".into())))
                                .size(10.5)
                                .color(egui::Color32::from_rgb(110, 116, 126)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("退出").clicked() {
                                self.quitting = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui.button("隐藏到托盘").clicked() {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                        });
                    });
                });
            });
    }
}

fn big_button(ui: &mut egui::Ui, title: &str, hint: &str, width: f32) -> egui::Response {
    let text = egui::RichText::new(format!("{title}\n{hint}")).size(13.0);
    ui.add_sized([width, 58.0], egui::Button::new(text).corner_radius(9.0))
}

fn copy_to_clipboard(img: &image::RgbaImage) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(arboard::ImageData {
        width: img.width() as usize,
        height: img.height() as usize,
        bytes: std::borrow::Cow::Borrowed(img.as_raw()),
    })
    .map_err(|e| e.to_string())
}

fn save_png(img: &image::RgbaImage, path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    image::save_buffer(
        path,
        img.as_raw(),
        img.width(),
        img.height(),
        image::ColorType::Rgba8,
    )
    .map_err(|e| e.to_string())
}

/// Programmatically drawn app icon (a framed crop mark).
pub fn app_icon(size: u32) -> egui::IconData {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    for y in 0..size {
        for x in 0..size {
            let u = (x as f32 + 0.5) / s;
            let v = (y as f32 + 0.5) / s;
            let r = 0.22f32;
            let du = u.min(1.0 - u);
            let dv = v.min(1.0 - v);
            let inside = if du >= r || dv >= r {
                true
            } else {
                let dx = r - du;
                let dy = r - dv;
                (dx * dx + dy * dy).sqrt() <= r
            };
            let i = ((y * size + x) * 4) as usize;
            if !inside {
                rgba[i + 3] = 0;
                continue;
            }
            let ring = du < 0.075 || dv < 0.075;
            let (cr, cg, cb) = if ring {
                (240, 244, 250)
            } else {
                (56, 132, 255)
            };
            rgba[i] = cr;
            rgba[i + 1] = cg;
            rgba[i + 2] = cb;
            rgba[i + 3] = 255;
        }
    }
    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}

fn build_tray() -> (Option<tray_icon::TrayIcon>, Option<MenuHandles>) {
    let menu = muda::Menu::new();
    let region = muda::MenuItem::new("区域截图", true, None);
    let full = muda::MenuItem::new("全屏截图", true, None);
    let panel = muda::MenuItem::new("显示主面板", true, None);
    let quit = muda::MenuItem::new("退出", true, None);
    for item in [&region, &full, &panel, &quit] {
        let _ = menu.append(item);
    }
    let icon = app_icon(32);
    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip("RustShot 截图")
        .with_icon(
            match tray_icon::Icon::from_rgba(icon.rgba, icon.width, icon.height) {
                Ok(i) => i,
                Err(_) => return (None, None),
            },
        )
        .with_menu(Box::new(menu))
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

impl eframe::App for App {
    /// Runs even while the window is hidden/minimized, so hotkeys keep working.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Closing the panel window goes to the tray instead of quitting.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            self.status = "已最小化到托盘：双击托盘图标截图，右键菜单可退出".to_owned();
            crate::util::log("panel close -> minimized to tray");
        }
        self.poll_save();
        self.poll_dir();
        self.pump_worker(ctx);
        self.pump_tray(ctx);
        if let Some(start) = self.start.take() {
            match start {
                Start::Panel => {}
                Start::Region => self.begin_capture(ctx, Tool::Select),
                Start::Full => self.capture_full(),
            }
        }
        if self.session.is_some() && !self.paint_hidden_sent {
            // The overlay lives in a child viewport of this window: keep painting
            // even when the panel is hidden.
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            self.paint_hidden_sent = true;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.run_overlay(&ctx);
        self.panel(ui);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.086, 0.09, 0.102, 1.0]
    }
}
