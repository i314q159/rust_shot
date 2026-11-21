//! eframe application: control panel, overlay orchestration, clipboard / file plumbing.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use crate::capture::{self, PhysRect};
use crate::i18n::{tr, tr1};
use crate::overlay::{self, Export, Outcome, Session, Tool};
use crate::update::{self, Status as UpdateStatus};
use crate::worker::{Worker, WorkerEvent};

/// Window title of the control panel (also used to find the HWND).
pub fn panel_title() -> &'static str {
    crate::i18n::tr("RustShot 截图工具", "RustShot Screenshot")
}

/// Drawn icon assets (see `assets/icon/preview.png`).
const ICON_32: &[u8] = include_bytes!("../assets/icon/rustshot-32.png");
const ICON_64: &[u8] = include_bytes!("../assets/icon/rustshot-64.png");
const ICON_256: &[u8] = include_bytes!("../assets/icon/rustshot-256.png");

/// The licence text shipped with the binary and shown in the Help menu.
const LICENSE_TEXT: &str = include_str!("../LICENSE-MIT.md");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_AUTHOR: &str = "i314q159";

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
    /// Start straight in the tray (used by the installer's autostart entry).
    Tray,
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
    /// Where the panel lives while it is parked off screen (hidden to the tray).
    panel_park: Option<crate::win::Parked>,
    /// RequestPaintWhileHidden only needs to be sent once per capture.
    paint_hidden_sent: bool,
    /// Set while the save dialog runs on its own thread.
    save_rx: Option<std::sync::mpsc::Receiver<SaveResult>>,
    /// Set while the "pick folder" dialog runs on its own thread.
    dir_rx: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
    /// Tray menu clicks arrive on the OS side; queue them here.
    menu_events: Arc<Mutex<Vec<muda::MenuId>>>,
    /// Tray icon clicks arrive on the OS side; queue them here.
    tray_events: Arc<Mutex<Vec<tray_icon::TrayIconEvent>>>,
    /// Help → 开源声明 window is open.
    show_license: bool,
    /// Top bar theme choice (system / light / dark), persisted in config.txt.
    theme_pref: crate::theme::Preference,
    /// Top bar language choice (system / zh / en), persisted in config.txt.
    lang_pref: crate::i18n::Preference,
    /// Latest GitHub update check result shown in the panel / dialog.
    update: UpdateStatus,
    /// Set while the update request runs on its own thread.
    update_rx: Option<std::sync::mpsc::Receiver<UpdateStatus>>,
    /// Update dialog is open.
    show_update: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Start) -> Self {
        let _ = UI_CTX.set(cc.egui_ctx.clone());
        crate::fonts::install_egui_font(&cc.egui_ctx);
        let cfg = crate::config::Config::load();
        crate::i18n::set(crate::i18n::resolve(cfg.lang));
        let dark = crate::theme::resolve(cfg.theme);
        crate::theme::apply(&cc.egui_ctx, dark);
        crate::util::log(&format!(
            "theme -> {} ({}), lang -> {} ({})",
            if dark { "dark" } else { "light" },
            cfg.theme.key(),
            crate::i18n::lang().key(),
            cfg.lang.key()
        ));
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(10.0, 6.0);
        });

        let worker = Worker::spawn();
        let (tray, menu) = build_tray();
        crate::util::log(&format!("app created, tray={}", tray.is_some()));

        // Tray start (plain launch / autostart): move the window off screen
        // before the first frame, so nothing flashes on the desktop.
        let panel_park = if start == Start::Tray {
            let parked = crate::win::park_offscreen(panel_title());
            crate::util::log(&format!(
                "started in tray, parked={}",
                parked.as_ref().map(|p| p.rect).is_some()
            ));
            parked
        } else {
            None
        };

        // egui only paints on demand, so a tray click would otherwise sit in the
        // queue until the user happens to hover the panel. Queue the events and
        // wake the UI right away.
        let menu_events: Arc<Mutex<Vec<muda::MenuId>>> = Arc::new(Mutex::new(Vec::new()));
        let tray_events: Arc<Mutex<Vec<tray_icon::TrayIconEvent>>> =
            Arc::new(Mutex::new(Vec::new()));
        {
            let queue = menu_events.clone();
            muda::MenuEvent::set_event_handler(Some(move |ev: muda::MenuEvent| {
                if let Ok(mut q) = queue.lock() {
                    q.push(ev.id);
                }
                request_repaint();
            }));
        }
        {
            let queue = tray_events.clone();
            tray_icon::TrayIconEvent::set_event_handler(Some(
                move |ev: tray_icon::TrayIconEvent| {
                    if let Ok(mut q) = queue.lock() {
                        q.push(ev);
                    }
                    request_repaint();
                },
            ));
        }

        Self {
            worker,
            session: None,
            save_dir: cfg.save_dir,
            auto_save: cfg.auto_save,
            theme_pref: cfg.theme,
            lang_pref: cfg.lang,
            status: String::new(),
            tray,
            menu,
            start: Some(start),
            quitting: false,
            waiting_panel: false,
            restore_frames: 0,
            serial: 0,
            remiminimize: false,
            panel_park,
            paint_hidden_sent: false,
            save_rx: None,
            dir_rx: None,
            menu_events,
            tray_events,
            show_license: false,
            update: UpdateStatus::Idle,
            update_rx: None,
            show_update: false,
        }
    }

    /// Kick off (or restart) the GitHub update check.
    fn check_update(&mut self) {
        if self.update_rx.is_some() {
            return;
        }
        self.update = UpdateStatus::Checking;
        self.update_rx = Some(update::check_async());
        self.show_update = true;
        self.status = tr("正在检查更新…", "Checking for updates…").to_owned();
    }

    /// Re-read the Windows light/dark setting and restyle when it changed.
    ///
    /// `WM_SETTINGCHANGE` also fires for unrelated settings, so the value is
    /// read back and compared before touching the styles.
    fn refresh_theme(&mut self, ctx: &egui::Context) {
        // A pinned light/dark choice ignores the OS entirely.
        if self.theme_pref != crate::theme::Preference::System {
            return;
        }
        let dark = crate::theme::system_is_dark();
        if dark == crate::theme::is_dark() {
            return;
        }
        crate::theme::apply(ctx, dark);
        crate::util::log(&format!("theme -> {}", if dark { "dark" } else { "light" }));
        ctx.request_repaint();
    }

    /// Top bar choice: remember it, restyle immediately, write it to config.txt.
    fn set_theme(&mut self, ctx: &egui::Context, pref: crate::theme::Preference) {
        self.theme_pref = pref;
        let dark = crate::theme::resolve(pref);
        crate::theme::apply(ctx, dark);
        self.persist();
        crate::util::log(&format!(
            "theme pref -> {} ({})",
            pref.key(),
            if dark { "dark" } else { "light" }
        ));
        self.status = tr1("主题：{}", "Theme: {}", pref.label());
        ctx.request_repaint();
    }

    /// Top bar language choice: retitle the window and the tray, then save it.
    fn set_lang(&mut self, ctx: &egui::Context, pref: crate::i18n::Preference) {
        self.lang_pref = pref;
        crate::i18n::set(crate::i18n::resolve(pref));
        self.persist();
        // The tray menu lives on the OS side, so its labels have to be pushed.
        if let Some(m) = &self.menu {
            m.region.set_text(tr("区域截图", "Region capture"));
            m.full.set_text(tr("全屏截图", "Full screen"));
            m.panel.set_text(tr("显示主面板", "Show panel"));
            m.quit.set_text(tr("退出", "Quit"));
        }
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(tr("RustShot 截图", "RustShot")));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(panel_title().to_owned()));
        crate::util::log(&format!(
            "lang pref -> {} ({})",
            pref.key(),
            crate::i18n::lang().key()
        ));
        self.status = tr1("语言：{}", "Language: {}", crate::i18n::lang().label());
        ctx.request_repaint();
    }

    fn poll_update(&mut self) {
        let Some(rx) = self.update_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(status) => {
                match &status {
                    UpdateStatus::Newer(rel) => {
                        self.status = tr1(
                            "发现新版本 v{}，可在更新窗口查看",
                            "New version v{} is available, see the update window",
                            &rel.version,
                        );
                    }
                    UpdateStatus::Latest(v) => {
                        self.status = tr1(
                            "已是最新版本（GitHub 最新 v{}）",
                            "Up to date (GitHub latest v{})",
                            v,
                        );
                    }
                    UpdateStatus::Failed(e) => {
                        self.status = tr1("检查更新失败：{}", "Update check failed: {}", e)
                    }
                    _ => {}
                }
                self.update = status;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.update_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.update = UpdateStatus::Failed(
                    tr(
                        "检查更新的线程意外退出",
                        "the update thread exited unexpectedly",
                    )
                    .to_owned(),
                );
            }
        }
    }

    /// Folder picker on its own thread: `pick_folder` also owns a modal loop.
    fn pick_dir(&mut self) {
        if self.dir_rx.is_some() {
            self.status =
                tr("目录选择框已经打开了", "The folder picker is already open").to_owned();
            return;
        }
        let start = self.save_dir.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.dir_rx = Some(rx);
        self.status = tr("正在选择保存目录…", "Choosing a save folder…").to_owned();
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
                self.status = tr1("保存目录：{}", "Save folder: {}", self.save_dir.display());
            }
            Ok(None) => self.status = tr("已取消选择目录", "Folder selection cancelled").to_owned(),
            Err(std::sync::mpsc::TryRecvError::Empty) => self.dir_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status = tr("目录选择失败", "Folder selection failed").to_owned();
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
                self.status = tr1("已保存 {}", "Saved {}", path.display());
            }
            Ok(SaveResult::Failed(e)) => self.status = tr1("保存失败：{}", "Save failed: {}", e),
            Ok(SaveResult::Cancelled) => {
                self.status = tr("已取消保存", "Save cancelled").to_owned()
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.save_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status = tr(
                    "保存失败：对话框线程退出",
                    "Save failed: the dialog thread exited",
                )
                .to_owned();
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
                WorkerEvent::CaptureRegion => self.begin_capture(ctx, Tool::Select, false),
                WorkerEvent::CaptureFull => self.capture_full(),
                WorkerEvent::CaptureFullOverlay => self.begin_capture(ctx, Tool::Select, true),
                WorkerEvent::ShowPanel => self.show_panel(ctx),
                WorkerEvent::ThemeChanged => self.refresh_theme(ctx),
                WorkerEvent::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    /// Hide the panel to the tray. The window is *not* minimized: it is parked
    /// outside every monitor, so eframe can still host the overlay viewport and
    /// a capture started from the tray never has to bring the panel back
    /// (which used to flash it on screen and steal focus).
    fn hide_to_tray(&mut self, ctx: &egui::Context) {
        if self.panel_park.is_some() {
            return;
        }
        match crate::win::park_offscreen(panel_title()) {
            Some(parked) => {
                self.panel_park = Some(parked);
                crate::util::log(&format!("panel parked off screen at {:?}", parked.rect));
                ctx.request_repaint();
            }
            // No usable window: fall back to a plain minimize.
            None => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
        }
    }

    fn show_panel(&mut self, ctx: &egui::Context) {
        if let Some(parked) = self.panel_park.take() {
            crate::win::unpark(panel_title(), parked);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    fn pump_tray(&mut self, ctx: &egui::Context) {
        let mut action: Option<u8> = None;
        let menu_ids = self
            .menu_events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        let tray_events = self
            .tray_events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        if let Some(m) = &self.menu {
            for id in menu_ids {
                if id == m.region.id() {
                    action = Some(0);
                } else if id == m.full.id() {
                    action = Some(1);
                } else if id == m.panel.id() {
                    action = Some(2);
                } else if id == m.quit.id() {
                    action = Some(3);
                }
            }
        }
        for ev in tray_events {
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
            Some(0) => self.begin_capture(ctx, Tool::Select, false),
            Some(1) => self.begin_capture(ctx, Tool::Select, true),
            Some(2) => self.show_panel(ctx),
            Some(3) => {
                self.quitting = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {}
        }
    }

    /// Freeze the monitor under the pointer and open the annotation overlay.
    ///
    /// `full_frame` preselects the whole screen (F2): Enter copies it as-is, the
    /// tools annotate on top, and a drag narrows it down to a region.
    fn begin_capture(&mut self, ctx: &egui::Context, tool: Tool, full_frame: bool) {
        let monitor = capture::monitor_at_cursor();
        let hover = if full_frame {
            None
        } else {
            capture::window_rect_at_cursor()
        };
        crate::util::log(&format!(
            "begin_capture tool={tool:?} full={full_frame} monitor={monitor:?} hover={hover:?}"
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
                    full_frame,
                ));
                self.status.clear();
                // eframe cannot create child viewports from a hidden or
                // minimized window, so the panel has to be mapped before the
                // overlay appears. When it is parked off screen that already
                // holds and nothing visible happens; only a panel the user
                // minimized by hand has to be restored for a frame.
                if !crate::win::is_ready(panel_title()) {
                    if self.panel_park.is_some() {
                        crate::win::show_without_activation(panel_title());
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        self.remiminimize = true;
                    }
                    self.waiting_panel = true;
                    self.restore_frames = 2;
                }
            }
            None => {
                self.status = tr(
                    "截屏失败：无法读取屏幕内容",
                    "Capture failed: cannot read the screen",
                )
                .to_owned()
            }
        }
    }

    /// --full from the command line: freeze, render the whole monitor and copy
    /// it, without opening the overlay (scripts have nothing to click).
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
            None => {
                self.status = tr(
                    "截屏失败：无法读取屏幕内容",
                    "Capture failed: cannot read the screen",
                )
                .to_owned()
            }
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
            if !crate::win::is_ready(panel_title()) {
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
            self.status = tr(
                "创建截图覆盖层失败，请重试",
                "Could not create the capture overlay, try again",
            )
            .to_owned();
        } else if let Some(o) = outcome {
            self.session = None;
            self.deliver(o);
        } else if cancelled {
            self.session = None;
            self.status = tr("已取消", "Cancelled").to_owned();
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
                    Ok(()) => tr("已复制到剪贴板", "Copied to clipboard").to_owned(),
                    Err(e) => tr1("复制失败：{}", "Copy failed: {}", e),
                };
                if self.auto_save {
                    match self.save_auto(&img) {
                        Ok(p) => msg.push_str(&tr1("，已保存 {}", ", saved {}", p.display())),
                        Err(e) => {
                            msg.push_str(&tr1("，自动保存失败：{}", ", auto-save failed: {}", e))
                        }
                    }
                }
                self.status = msg;
            }
            Export::Save => {
                if self.save_rx.is_some() {
                    self.status =
                        tr("保存对话框已经打开了", "The save dialog is already open").to_owned();
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
                self.status = tr("正在选择保存位置…", "Choosing where to save…").to_owned();
                // The dialog owns a nested message loop and a COM apartment:
                // running it inside our paint pass froze the window, and it also
                // ended up hidden behind the always-on-top overlay.
                std::thread::Builder::new()
                    .name("rustshot-save".into())
                    .spawn(move || {
                        let picked = rfd::FileDialog::new()
                            .set_directory(&dir)
                            .set_file_name(file_name)
                            .add_filter(tr("PNG 图片", "PNG image"), &["png"])
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
            theme: self.theme_pref,
            lang: self.lang_pref,
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

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(crate::theme::bar())
            .inner_margin(egui::Margin::symmetric(10, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    // 这是截图工具，没有"文件"概念：菜单就叫截图，退出/隐藏交给底栏和托盘。
                    ui.menu_button(tr("截图", "Capture"), |ui| {
                        if ui
                            .button(tr("区域截图\tF1", "Region capture\tF1"))
                            .clicked()
                        {
                            let ctx = ui.ctx().clone();
                            self.begin_capture(&ctx, Tool::Select, false);
                            ui.close();
                        }
                        if ui.button(tr("全屏截图\tF2", "Full screen\tF2")).clicked() {
                            let ctx = ui.ctx().clone();
                            self.begin_capture(&ctx, Tool::Select, true);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(tr("打开保存目录", "Open save folder")).clicked() {
                            let _ = std::fs::create_dir_all(&self.save_dir);
                            let _ = std::process::Command::new("explorer")
                                .arg(&self.save_dir)
                                .spawn();
                            ui.close();
                        }
                    });
                    ui.menu_button(tr("主题", "Theme"), |ui| {
                        let system_dark = crate::theme::system_is_dark();
                        for pref in [
                            crate::theme::Preference::System,
                            crate::theme::Preference::Light,
                            crate::theme::Preference::Dark,
                        ] {
                            let label = match pref {
                                crate::theme::Preference::System => tr1(
                                    "跟随系统（当前{}）",
                                    "Follow system (now {})",
                                    if system_dark {
                                        tr("深色", "dark")
                                    } else {
                                        tr("浅色", "light")
                                    },
                                ),
                                other => other.label().to_owned(),
                            };
                            if ui
                                .selectable_label(self.theme_pref == pref, label)
                                .clicked()
                            {
                                let ctx = ui.ctx().clone();
                                self.set_theme(&ctx, pref);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button(tr("语言", "Language"), |ui| {
                        let system = crate::i18n::detect();
                        for pref in [
                            crate::i18n::Preference::System,
                            crate::i18n::Preference::Zh,
                            crate::i18n::Preference::En,
                        ] {
                            let label = match pref {
                                crate::i18n::Preference::System => tr1(
                                    "跟随系统（当前{}）",
                                    "Follow system (now {})",
                                    system.label(),
                                ),
                                other => other.label(),
                            };
                            if ui.selectable_label(self.lang_pref == pref, label).clicked() {
                                let ctx = ui.ctx().clone();
                                self.set_lang(&ctx, pref);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button(tr("帮助", "Help"), |ui| {
                        if ui.button(tr("检查更新", "Check for updates")).clicked() {
                            self.check_update();
                            ui.close();
                        }
                        if ui
                            .button(tr("打开发布页（GitHub）", "Open releases page (GitHub)"))
                            .clicked()
                        {
                            update::open_page(update::RELEASES_URL);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(tr("开源声明（MIT）", "License (MIT)")).clicked() {
                            self.show_license = true;
                            ui.close();
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("v{APP_VERSION} · MIT"))
                                .size(10.5)
                                .color(crate::theme::text_faint()),
                        );
                        match self.update.clone() {
                            UpdateStatus::Newer(rel) => {
                                let badge = egui::Button::new(
                                    egui::RichText::new(tr1(
                                        "可更新 v{}",
                                        "Update v{}",
                                        rel.version,
                                    ))
                                    .size(10.5)
                                    .color(crate::theme::accent()),
                                )
                                .frame(false);
                                if ui
                                    .add(badge)
                                    .on_hover_text(tr(
                                        "点击查看更新内容",
                                        "Click to see what's new",
                                    ))
                                    .clicked()
                                {
                                    self.show_update = true;
                                }
                            }
                            UpdateStatus::Checking => {
                                ui.label(
                                    egui::RichText::new(tr("检查更新…", "Checking…"))
                                        .size(10.5)
                                        .color(crate::theme::text_faint()),
                                );
                            }
                            _ => {}
                        }
                    });
                });
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        self.update_dialog(ctx);
        if self.show_license {
            let mut open = true;
            egui::Window::new(tr("开源声明 · MIT License", "License · MIT"))
                .open(&mut open)
                .collapsible(false)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(if crate::i18n::is_zh() {
                            format!("RustShot v{APP_VERSION} · 作者 {APP_AUTHOR} · MIT License")
                        } else {
                            format!("RustShot v{APP_VERSION} · by {APP_AUTHOR} · MIT License")
                        })
                        .size(12.0)
                        .color(crate::theme::text_weak()),
                    );
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical()
                        .max_height(330.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(LICENSE_TEXT).monospace().size(11.5));
                        });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button(tr("复制全文", "Copy full text")).clicked() {
                            ui.ctx().copy_text(LICENSE_TEXT.to_owned());
                        }
                        ui.label(
                            egui::RichText::new(tr(
                                "协议全文：LICENSE-MIT.md",
                                "Full text: LICENSE-MIT.md",
                            ))
                            .size(11.0)
                            .color(crate::theme::text_faint()),
                        );
                    });
                });
            if !open {
                self.show_license = false;
            }
        }
    }

    /// Update dialog: version numbers, release notes and the download entry.
    fn update_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_update {
            return;
        }
        let mut open = true;
        let mut retry = false;
        let mut close = false;
        let mut page: Option<String> = None;
        let current = self.update.clone();
        let release_url = match &current {
            UpdateStatus::Newer(rel) => rel.url.clone(),
            _ => update::RELEASES_URL.to_owned(),
        };
        egui::Window::new(tr("检查更新", "Check for updates"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(if crate::i18n::is_zh() {
                        format!("当前版本 v{APP_VERSION} · 仓库 {}", update::REPO)
                    } else {
                        format!("Current v{APP_VERSION} · repo {}", update::REPO)
                    })
                    .size(11.0)
                    .color(crate::theme::text_faint()),
                );
                ui.add_space(6.0);
                match &current {
                    UpdateStatus::Idle => {
                        ui.label(
                            egui::RichText::new(tr("尚未检查更新", "Not checked yet")).size(12.5),
                        );
                    }
                    UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new(tr(
                                    "正在连接 GitHub 检查最新版本…",
                                    "Connecting to GitHub…",
                                ))
                                .size(12.5),
                            );
                        });
                    }
                    UpdateStatus::Latest(version) => {
                        ui.label(
                            egui::RichText::new(tr1(
                                "已是最新版本（GitHub 最新 v{}）",
                                "Up to date (GitHub latest v{})",
                                version,
                            ))
                            .size(13.0)
                            .color(crate::theme::ok()),
                        );
                        ui.label(
                            egui::RichText::new(tr(
                                "有新版本发布时可以在这里直接打开发布页下载。",
                                "New releases can be opened from here.",
                            ))
                            .size(11.5)
                            .color(crate::theme::text_weak()),
                        );
                    }
                    UpdateStatus::Newer(rel) => {
                        ui.label(
                            egui::RichText::new(tr1(
                                "发现新版本 v{}",
                                "New version v{}",
                                &rel.version,
                            ))
                            .size(15.0)
                            .strong()
                            .color(crate::theme::ok()),
                        );
                        let mut meta = rel.name.clone();
                        if !rel.published.is_empty() {
                            if !meta.trim().is_empty() {
                                meta.push_str(" · ");
                            }
                            meta.push_str(tr("发布于 ", "released "));
                            meta.push_str(&rel.published);
                        }
                        if !meta.trim().is_empty() {
                            ui.label(
                                egui::RichText::new(meta)
                                    .size(11.0)
                                    .color(crate::theme::text_faint()),
                            );
                        }
                        ui.add_space(4.0);
                        if rel.notes.trim().is_empty() {
                            ui.label(
                                egui::RichText::new(tr(
                                    "该版本没有填写更新说明。",
                                    "This release has no notes.",
                                ))
                                .size(11.5)
                                .color(crate::theme::text_weak()),
                            );
                        } else {
                            egui::Frame::NONE
                                .fill(crate::theme::hint())
                                .stroke(egui::Stroke::new(1.0, crate::theme::line()))
                                .corner_radius(6.0)
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    egui::ScrollArea::vertical()
                                        .max_height(180.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    egui::RichText::new(&rel.notes)
                                                        .monospace()
                                                        .size(11.0),
                                                )
                                                .wrap(),
                                            );
                                        });
                                });
                        }
                    }
                    UpdateStatus::Failed(err) => {
                        ui.label(
                            egui::RichText::new(tr("检查更新失败", "Update check failed"))
                                .size(13.0)
                                .color(crate::theme::warn()),
                        );
                        ui.label(
                            egui::RichText::new(err)
                                .size(11.5)
                                .color(crate::theme::text_weak()),
                        );
                        ui.label(
                            egui::RichText::new(tr(
                                "也可以直接打开发布页确认最新版本。",
                                "You can also open the releases page.",
                            ))
                            .size(11.0)
                            .color(crate::theme::text_faint()),
                        );
                    }
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let busy = matches!(current, UpdateStatus::Checking);
                    if ui
                        .add_enabled(!busy, egui::Button::new(tr("重新检查", "Check again")))
                        .clicked()
                    {
                        retry = true;
                    }
                    if ui.button(tr("打开发布页", "Open releases page")).clicked() {
                        page = Some(release_url.clone());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr("关闭", "Close")).clicked() {
                            close = true;
                        }
                    });
                });
            });
        if retry {
            self.check_update();
        }
        if close {
            open = false;
        }
        if let Some(url) = page {
            update::open_page(&url);
            open = false;
        }
        if !open {
            self.show_update = false;
        }
    }

    fn panel(&mut self, ui: &mut egui::Ui) {
        self.menu_bar(ui);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(crate::theme::bg())
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(egui::RichText::new("RustShot").size(24.0).strong());
                    ui.label(
                        egui::RichText::new(tr("截图 · 标注", "Screenshot · Annotate"))
                            .size(12.0)
                            .color(crate::theme::text_faint()),
                    );
                });
                ui.add_space(10.0);

                let bw = (ui.available_width() - 16.0) / 2.0;
                ui.horizontal(|ui| {
                    if big_button(ui, tr("区域截图", "Region capture"), "F1", bw).clicked() {
                        let ctx = ui.ctx().clone();
                        self.begin_capture(&ctx, Tool::Select, false);
                    }
                    if big_button(ui, tr("全屏截图", "Full screen"), "F2", bw).clicked() {
                        let ctx = ui.ctx().clone();
                        self.begin_capture(&ctx, Tool::Select, true);
                    }
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new(tr("保存目录", "Save folder"))
                        .size(12.5)
                        .color(crate::theme::text_weak()),
                )
                    .on_hover_text(match crate::config::path() {
                        Some(p) => tr1(
                            "配置保存在 {}\n可手动编辑 save_dir / auto_save / theme / lang",
                            "Config: {}\nedit save_dir / auto_save / theme / lang by hand",
                            p.display(),
                        ),
                        None => tr(
                            "配置保存在程序目录下的 config.txt",
                            "Config lives in config.txt next to the exe",
                        )
                        .to_owned(),
                    });
                ui.horizontal(|ui| {
                    // Reserve room for 更改 / 打开 / 重置, otherwise a long path
                    // pushes them out of the row.
                    let path_w = (ui.available_width() - 176.0).max(80.0);
                    ui.add_sized(
                        [path_w, 18.0],
                        egui::Label::new(
                            egui::RichText::new(self.save_dir.display().to_string())
                                .size(12.0)
                                .monospace(),
                        )
                        .truncate(),
                    );
                    if ui.small_button(tr("更改", "Change")).clicked() {
                        self.pick_dir();
                    }
                    if ui.small_button(tr("打开", "Open")).clicked() {
                        let _ = std::fs::create_dir_all(&self.save_dir);
                        let _ = std::process::Command::new("explorer")
                            .arg(&self.save_dir)
                            .spawn();
                    }
                    if ui
                        .small_button(tr("重置", "Reset"))
                        .on_hover_text(tr(
                            "恢复默认目录并写入配置文件",
                            "Restore the default folder and write it to config.txt",
                        ))
                        .clicked()
                    {
                        self.save_dir = crate::util::default_save_dir();
                        self.persist();
                        self.status = tr1(
                            "已重置为默认目录：{}",
                            "Reset to the default folder: {}",
                            self.save_dir.display(),
                        );
                    }
                });
                if ui
                    .checkbox(&mut self.auto_save, tr("复制到剪贴板时同时保存 PNG", "Also save a PNG when copying"))
                    .changed()
                {
                    self.persist();
                }

                ui.add_space(8.0);
                egui::Frame::NONE
                    .fill(crate::theme::hint())
                    .stroke(egui::Stroke::new(1.0, crate::theme::line()))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(
                                tr(
                                    "截图后：拖选区域 · 单击选中窗口 · Enter / Ctrl+C 复制 · Ctrl+S 另存为",
                                    "After starting: drag a region · click a window · Enter / Ctrl+C copies · Ctrl+S saves",
                                ),
                            )
                            .size(11.5)
                            .color(crate::theme::text_weak()),
                        );
                        ui.label(
                            egui::RichText::new(
                                tr(
                                    "标注：矩形 / 箭头 / 画笔 / 文字 / 马赛克 / 序号，Ctrl+Z 撤销，Ctrl+A 全选",
                                    "Tools: rect / arrow / pen / text / mosaic / number · Ctrl+Z undo · Ctrl+A select all",
                                ),
                            )
                            .size(11.5)
                            .color(crate::theme::text_weak()),
                        );
                        ui.label(
                            egui::RichText::new(tr(
                                    "全局热键：F1 区域截图 · F2 全屏（进标注）· Ctrl+Alt+Q 退出",
                                    "Hotkeys: F1 region · F2 full screen (annotate) · Ctrl+Alt+Q quit",
                                ))
                                .size(11.5)
                                .color(crate::theme::text_weak()),
                        );
                    });

                ui.add_space(8.0);
                if !self.status.is_empty() {
                    ui.label(
                        egui::RichText::new(&self.status)
                            .size(12.0)
                            .color(crate::theme::ok()),
                    );
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        if self.tray.is_none() {
                            ui.label(
                                egui::RichText::new(tr("托盘不可用", "Tray unavailable"))
                                    .size(11.0)
                                    .color(crate::theme::warn()),
                            );
                        }
                        ui.label(
                            egui::RichText::new(tr1(
                                    "字体：{}",
                                    "Font: {}",
                                    crate::fonts::cjk()
                                        .map(|f| f.label.clone())
                                        .unwrap_or_else(|| tr("系统默认", "system default").into()),
                                ))
                                .size(10.5)
                                .color(crate::theme::text_faint()),
                        );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(tr("退出", "Quit")).clicked() {
                                self.quitting = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui.button(tr("隐藏到托盘", "Hide to tray")).clicked() {
                                let ctx = ui.ctx().clone();
                                self.hide_to_tray(&ctx);
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

/// App icon, decoded from the drawn PNG assets in `assets/icon`.
pub fn app_icon(size: u32) -> egui::IconData {
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

fn build_tray() -> (Option<tray_icon::TrayIcon>, Option<MenuHandles>) {
    let menu = muda::Menu::new();
    let region = muda::MenuItem::new(tr("区域截图", "Region capture"), true, None);
    let full = muda::MenuItem::new(tr("全屏截图", "Full screen"), true, None);
    let panel = muda::MenuItem::new(tr("显示主面板", "Show panel"), true, None);
    let quit = muda::MenuItem::new(tr("退出", "Quit"), true, None);
    for item in [&region, &full, &panel, &quit] {
        let _ = menu.append(item);
    }
    let icon = app_icon(32);
    let tray = tray_icon::TrayIconBuilder::new()
        .with_tooltip(tr("RustShot 截图", "RustShot"))
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

impl eframe::App for App {
    /// Runs even while the window is hidden/minimized, so hotkeys keep working.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Closing the panel window goes to the tray instead of quitting.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_to_tray(ctx);
            self.status = tr(
                "已隐藏到托盘：F1 截图 · 右键托盘图标可退出",
                "Hidden to tray: F1 captures · right-click the tray icon to quit",
            )
            .to_owned();
            crate::util::log("panel close -> hidden to tray");
        }
        self.poll_save();
        self.poll_dir();
        self.poll_update();
        self.pump_worker(ctx);
        self.pump_tray(ctx);
        if self.panel_park.is_some() {
            // eframe re-applies winit's window styles on the first frames, which
            // would put the parked panel back into the taskbar.
            crate::win::hide_from_taskbar(panel_title());
        }
        if let Some(start) = self.start.take() {
            match start {
                Start::Panel => {}
                Start::Region => self.begin_capture(ctx, Tool::Select, false),
                Start::Full => self.capture_full(),
                Start::Tray => {
                    self.hide_to_tray(ctx);
                    self.status = tr(
                        "已在托盘常驻：F1 截图 · F2 全屏",
                        "Running in the tray: F1 region · F2 full screen",
                    )
                    .to_owned();
                    crate::util::log("started in tray");
                }
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
        self.dialogs(&ctx);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        // Same colour as the panel, so resizing never flashes the opposite theme.
        let c = visuals.panel_fill;
        [
            c.r() as f32 / 255.0,
            c.g() as f32 / 255.0,
            c.b() as f32 / 255.0,
            1.0,
        ]
    }
}
