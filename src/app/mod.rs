pub(crate) mod worker;

use crate::config::i18n::strings;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use crate::app::worker::{Worker, WorkerEvent};
use crate::capture::overlay::{self, Export, Outcome, Session, Tool};
use crate::capture::{self, PhysRect};
use crate::config::i18n::{tr, tr1};
use crate::ui::tray::{MenuHandles, build_tray};
use crate::update::{self, Status as UpdateStatus};

pub fn panel_title() -> &'static str {
    crate::config::i18n::tr(strings::RUSTSHOT_SCREENSHOT)
}

pub(crate) const LICENSE_TEXT: &str = include_str!("../../LICENSE-MIT.md");
pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const APP_AUTHOR: &str = "i314q159";

static UI_CTX: OnceLock<egui::Context> = OnceLock::new();

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

    Tray,

    Check,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Install {
    Idle,
    Downloading { got: u64, total: u64 },
    Verifying,
    Launching,
    Failed(String),
}

pub(crate) enum InstallEvent {
    Progress(u64, u64),
    Verifying,
    Failed(String),
    Done,
}

pub(crate) enum SaveResult {
    Saved(PathBuf),
    Failed(String),
    Cancelled,
}

pub struct App {
    pub(crate) worker: Worker,
    pub(crate) session: Option<Session>,
    pub(crate) save_dir: PathBuf,
    pub(crate) auto_save: bool,
    pub(crate) status: String,
    pub(crate) tray: Option<tray_icon::TrayIcon>,
    pub(crate) menu: Option<MenuHandles>,
    pub(crate) start: Option<Start>,

    pub(crate) quitting: bool,

    pub(crate) waiting_panel: bool,

    pub(crate) restore_frames: u8,

    pub(crate) serial: u64,

    pub(crate) remiminimize: bool,

    pub(crate) panel_park: Option<crate::platform::win::Parked>,

    pub(crate) paint_hidden_sent: bool,

    pub(crate) save_rx: Option<std::sync::mpsc::Receiver<SaveResult>>,

    pub(crate) dir_rx: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,

    pub(crate) menu_events: Arc<Mutex<Vec<muda::MenuId>>>,

    pub(crate) tray_events: Arc<Mutex<Vec<tray_icon::TrayIconEvent>>>,

    pub(crate) show_license: bool,

    pub(crate) theme_pref: crate::theme::Preference,

    pub(crate) lang_pref: crate::config::i18n::Preference,

    pub(crate) update: UpdateStatus,

    pub(crate) update_rx: Option<std::sync::mpsc::Receiver<UpdateStatus>>,

    pub(crate) show_update: bool,

    pub(crate) auto_update: crate::update::AutoUpdate,

    pub(crate) proxy: crate::config::proxy::Proxy,

    pub(crate) proxy_host: String,

    pub(crate) proxy_port: String,

    pub(crate) proxy_mode: u8,

    pub(crate) proxy_bad: bool,

    pub(crate) show_proxy: bool,

    pub(crate) install: Install,

    pub(crate) install_rx: Option<std::sync::mpsc::Receiver<InstallEvent>>,

    pub(crate) copy_rx: Option<std::sync::mpsc::Receiver<String>>,

    pub(crate) reveal_on_update: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Start) -> Self {
        let _ = UI_CTX.set(cc.egui_ctx.clone());
        crate::config::fonts::install_egui_font(&cc.egui_ctx);
        let cfg = crate::config::Config::load();

        if !crate::config::exists() {
            let _ = cfg.save();
        }
        crate::config::i18n::set(crate::config::i18n::resolve(cfg.lang));
        let dark = crate::theme::resolve(cfg.theme);
        crate::theme::apply(&cc.egui_ctx, dark);
        crate::platform::util::log(&format!(
            "theme -> {} ({}), lang -> {} ({})",
            if dark { "dark" } else { "light" },
            cfg.theme.key(),
            crate::config::i18n::lang().key(),
            cfg.lang.key()
        ));
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(10.0, 6.0);

            style.spacing.scroll.floating = false;
            style.spacing.scroll.foreground_color = true;
            style.spacing.scroll.bar_width = 9.0;
            style.spacing.scroll.bar_inner_margin = 3.0;
        });

        let worker = Worker::spawn();
        let (tray, menu) = build_tray();
        crate::platform::util::log(&format!("app created, tray={}", tray.is_some()));

        let panel_park = if matches!(start, Start::Tray | Start::Check) {
            let parked = crate::platform::win::park_offscreen(panel_title());
            crate::platform::util::log(&format!(
                "started in tray, parked={}",
                parked.as_ref().map(|p| p.rect).is_some()
            ));
            parked
        } else {
            None
        };

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
            auto_update: cfg.auto_update,
            proxy: cfg.proxy.clone(),
            proxy_host: match &cfg.proxy {
                crate::config::proxy::Proxy::Manual { host, .. } => host.clone(),
                _ => String::new(),
            },
            proxy_port: match &cfg.proxy {
                crate::config::proxy::Proxy::Manual { port, .. } => port.to_string(),
                _ => String::new(),
            },
            proxy_mode: match cfg.proxy {
                crate::config::proxy::Proxy::System => 0,
                crate::config::proxy::Proxy::Direct => 1,
                crate::config::proxy::Proxy::Manual { .. } => 2,
            },
            proxy_bad: false,
            show_proxy: false,
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
            install: Install::Idle,
            install_rx: None,
            copy_rx: None,
            reveal_on_update: false,
        }
    }

    pub(crate) fn check_update(&mut self) {
        if self.update_rx.is_some() {
            return;
        }
        self.update = UpdateStatus::Checking;
        self.update_rx = Some(update::check_async());
        self.show_update = true;
        self.status = tr(strings::CHECKING_FOR_UPDATES).to_owned();
    }

    fn refresh_theme(&mut self, ctx: &egui::Context) {
        if self.theme_pref != crate::theme::Preference::System {
            return;
        }
        let dark = crate::theme::system_is_dark();
        if dark == crate::theme::is_dark() {
            return;
        }
        crate::theme::apply(ctx, dark);
        crate::platform::util::log(&format!("theme -> {}", if dark { "dark" } else { "light" }));
        ctx.request_repaint();
    }

    pub(crate) fn set_theme(&mut self, ctx: &egui::Context, pref: crate::theme::Preference) {
        self.theme_pref = pref;
        let dark = crate::theme::resolve(pref);
        crate::theme::apply(ctx, dark);
        self.persist();
        crate::platform::util::log(&format!(
            "theme pref -> {} ({})",
            pref.key(),
            if dark { "dark" } else { "light" }
        ));
        self.status = tr1(strings::THEME, pref.label());
        ctx.request_repaint();
    }

    pub(crate) fn set_lang(&mut self, ctx: &egui::Context, pref: crate::config::i18n::Preference) {
        self.lang_pref = pref;
        crate::config::i18n::set(crate::config::i18n::resolve(pref));
        self.persist();

        if let Some(m) = &self.menu {
            m.region.set_text(tr(strings::REGION_CAPTURE));
            m.full.set_text(tr(strings::FULL_SCREEN));
            m.panel.set_text(tr(strings::SHOW_PANEL));
            m.quit.set_text(tr(strings::QUIT));
        }
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(tr(strings::RUSTSHOT)));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(panel_title().to_owned()));
        crate::platform::util::log(&format!(
            "lang pref -> {} ({})",
            pref.key(),
            crate::config::i18n::lang().key()
        ));
        self.status = tr1(strings::LANGUAGE, crate::config::i18n::lang().label());
        ctx.request_repaint();
    }

    pub(crate) fn manual_proxy(&self) -> Option<crate::config::proxy::Proxy> {
        let host = self.proxy_host.trim();
        let port = self
            .proxy_port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)?;
        if host.is_empty() {
            return None;
        }
        Some(crate::config::proxy::Proxy::Manual {
            host: host.to_owned(),
            port,
        })
    }

    pub(crate) fn apply_proxy(&mut self, mode: u8) {
        self.proxy_mode = mode;
        match mode {
            1 => {
                self.proxy = crate::config::proxy::Proxy::Direct;
                self.proxy_bad = false;
            }
            2 => {
                if let Some(parsed) = self.manual_proxy() {
                    self.proxy = parsed;
                    self.proxy_bad = false;
                } else {
                    self.proxy_bad = true;
                    self.status =
                        tr(strings::INVALID_PROXY_ADDRESS_KEEPING_THE_CURRENT_ONE).to_owned();
                    return;
                }
            }
            _ => {
                self.proxy = crate::config::proxy::Proxy::System;
                self.proxy_bad = false;
            }
        }
        match &self.proxy {
            crate::config::proxy::Proxy::Manual { host, port } => {
                self.proxy_host = host.clone();
                self.proxy_port = port.to_string();
            }
            _ => {
                self.proxy_host.clear();
                self.proxy_port.clear();
            }
        }
        self.persist();
        crate::platform::util::log(&format!("proxy -> {}", self.proxy.key()));
        self.status = tr1(strings::PROXY, self.proxy.label());
    }

    pub(crate) fn start_install(&mut self) {
        if self.install_rx.is_some() {
            return;
        }
        let UpdateStatus::Newer(rel) = self.update.clone() else {
            return;
        };
        self.install = Install::Downloading { got: 0, total: 0 };
        self.status = tr(strings::DOWNLOADING_THE_UPDATE).to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        self.install_rx = Some(rx);
        crate::platform::util::log(&format!("update: downloading v{}", rel.version));

        let _ = std::thread::Builder::new()
            .name("rustshot-update-dl".into())
            .spawn(move || {
                let asset = crate::update::setup_name(&rel.version);
                let dest = crate::update::update_dir().join(&asset);
                let result = (|| -> Result<(), String> {
                    let expected = crate::update::expected_sha(&rel.version, &asset)?;
                    let url = crate::update::asset_url(&rel.version, &asset);
                    let _ = tx.send(InstallEvent::Progress(0, 0));

                    let mut attempt = 0;
                    while let Err(e) = crate::update::download(&url, &dest, |got, total| {
                        let _ = tx.send(InstallEvent::Progress(got, total));
                        crate::app::request_repaint();
                    }) {
                        attempt += 1;
                        if attempt >= 2 {
                            return Err(e);
                        }
                        crate::platform::util::log(&format!(
                            "update: download failed, retrying: {e}"
                        ));
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        let _ = tx.send(InstallEvent::Progress(0, 0));
                    }
                    let _ = tx.send(InstallEvent::Verifying);
                    crate::app::request_repaint();
                    let actual = crate::update::sha256_file(&dest)?;
                    if actual != expected {
                        return Err(
                            tr(strings::THE_DOWNLOAD_DOES_NOT_MATCH_SHA256SUMS_TXT).to_owned()
                        );
                    }
                    crate::update::apply::spawn_helper(&dest)
                })();
                match result {
                    Ok(()) => {
                        let _ = tx.send(InstallEvent::Done);
                    }
                    Err(e) => {
                        crate::platform::util::log(&format!("update: install aborted: {e}"));
                        let _ = tx.send(InstallEvent::Failed(e));
                    }
                }
                crate::app::request_repaint();
            });
    }

    fn poll_install(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.install_rx.take() else {
            return;
        };
        let mut finished = false;
        loop {
            match rx.try_recv() {
                Ok(InstallEvent::Progress(got, total)) => {
                    self.install = Install::Downloading { got, total }
                }
                Ok(InstallEvent::Verifying) => self.install = Install::Verifying,
                Ok(InstallEvent::Failed(e)) => {
                    self.status = tr1(strings::UPDATE_FAILED, &e);
                    self.install = Install::Failed(e);
                }
                Ok(InstallEvent::Done) => {
                    self.install = Install::Launching;
                    finished = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.install_rx = Some(rx);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    if matches!(
                        self.install,
                        Install::Downloading { .. } | Install::Verifying
                    ) {
                        self.install = Install::Failed(
                            tr(strings::THE_UPDATE_THREAD_EXITED_UNEXPECTEDLY_2).to_owned(),
                        );
                    }
                    break;
                }
            }
        }
        if finished {
            self.quit_for_update(ctx);
        }
    }

    fn quit_for_update(&mut self, ctx: &egui::Context) {
        crate::platform::util::log("update: verified, quitting so the helper can install");
        crate::app::worker::release_instance();
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn poll_update(&mut self) {
        let Some(rx) = self.update_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(status) => {
                match &status {
                    UpdateStatus::Newer(rel)
                        if self.auto_update == crate::update::AutoUpdate::Silent =>
                    {
                        self.status =
                            tr1(strings::NEW_VERSION_V_FOUND_UPDATING_SILENTLY, &rel.version);
                    }
                    UpdateStatus::Newer(rel) => {
                        self.status = tr1(
                            strings::NEW_VERSION_V_IS_AVAILABLE_SEE_THE_UPDATE_WINDOW,
                            &rel.version,
                        );
                    }
                    UpdateStatus::Latest(v) => {
                        self.status = tr1(strings::UP_TO_DATE_GITHUB_LATEST_V, v);
                    }
                    UpdateStatus::Failed(e) => self.status = tr1(strings::UPDATE_CHECK_FAILED, e),
                    _ => {}
                }
                let auto = self.auto_update == crate::update::AutoUpdate::Silent
                    && matches!(status, UpdateStatus::Newer(_));
                self.update = status;
                if auto {
                    self.start_install();
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.update_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.update = UpdateStatus::Failed(
                    tr(strings::THE_UPDATE_THREAD_EXITED_UNEXPECTEDLY).to_owned(),
                );
            }
        }
    }

    pub(crate) fn pick_dir(&mut self) {
        if self.dir_rx.is_some() {
            self.status = tr(strings::THE_FOLDER_PICKER_IS_ALREADY_OPEN).to_owned();
            return;
        }
        let start = self.save_dir.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.dir_rx = Some(rx);
        self.status = tr(strings::CHOOSING_A_SAVE_FOLDER).to_owned();
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
                self.status = tr1(strings::SAVE_FOLDER, self.save_dir.display());
            }
            Ok(None) => self.status = tr(strings::FOLDER_SELECTION_CANCELLED).to_owned(),
            Err(std::sync::mpsc::TryRecvError::Empty) => self.dir_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status = tr(strings::FOLDER_SELECTION_FAILED).to_owned();
            }
        }
    }

    fn poll_copy(&mut self) {
        let Some(rx) = self.copy_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(msg) => self.status = msg,
            Err(std::sync::mpsc::TryRecvError::Empty) => self.copy_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status = tr(strings::THE_COPY_THREAD_EXITED_UNEXPECTEDLY).to_owned();
            }
        }
    }

    fn poll_save(&mut self) {
        let Some(rx) = self.save_rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(SaveResult::Saved(path)) => {
                crate::platform::util::log(&format!("saved {}", path.display()));
                self.status = tr1(strings::SAVED_2, path.display());
            }
            Ok(SaveResult::Failed(e)) => self.status = tr1(strings::SAVE_FAILED, e),
            Ok(SaveResult::Cancelled) => self.status = tr(strings::SAVE_CANCELLED).to_owned(),
            Err(std::sync::mpsc::TryRecvError::Empty) => self.save_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.status = tr(strings::SAVE_FAILED_THE_DIALOG_THREAD_EXITED).to_owned();
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
                WorkerEvent::CheckUpdate => {
                    self.reveal_on_update = true;
                    self.check_update();
                }
                WorkerEvent::ThemeChanged => self.refresh_theme(ctx),
                WorkerEvent::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    pub(crate) fn hide_to_tray(&mut self, ctx: &egui::Context) {
        if self.panel_park.is_some() {
            return;
        }
        match crate::platform::win::park_offscreen(panel_title()) {
            Some(parked) => {
                self.panel_park = Some(parked);
                crate::platform::util::log(&format!(
                    "panel parked off screen at {:?}",
                    parked.rect
                ));
                ctx.request_repaint();
            }

            None => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
        }
    }

    fn show_panel(&mut self, ctx: &egui::Context) {
        if let Some(parked) = self.panel_park.take() {
            crate::platform::win::unpark(panel_title(), parked);
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

    pub(crate) fn begin_capture(&mut self, ctx: &egui::Context, tool: Tool, full_frame: bool) {
        let monitor = capture::monitor_at_cursor();
        let hover = if full_frame {
            None
        } else {
            capture::window_rect_at_cursor()
        };
        crate::platform::util::log(&format!(
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

                if !crate::platform::win::is_ready(panel_title()) {
                    if self.panel_park.is_some() {
                        crate::platform::win::show_without_activation(panel_title());
                    } else {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        self.remiminimize = true;
                    }
                    self.waiting_panel = true;
                    self.restore_frames = 2;
                }
            }
            None => self.status = tr(strings::CAPTURE_FAILED_CANNOT_READ_THE_SCREEN).to_owned(),
        }
    }

    pub(crate) fn capture_full(&mut self) {
        if self.copy_rx.is_some() {
            self.status = tr(strings::COPYING_THE_IMAGE).to_owned();
            return;
        }
        let monitor = capture::monitor_at_cursor();
        let dir = self.save_dir.clone();
        let auto_save = self.auto_save;
        let (tx, rx) = std::sync::mpsc::channel();
        self.copy_rx = Some(rx);
        self.status = tr(strings::COPYING_THE_IMAGE).to_owned();
        std::thread::Builder::new()
            .name("rustshot-capture".into())
            .spawn(move || {
                let msg = match capture::capture(monitor) {
                    Some(frame) => {
                        let rect = PhysRect::new(0, 0, frame.w as i32, frame.h as i32);
                        let img = crate::capture::shapes::render(&frame, rect, &[]);
                        copy_and_save(&img, &dir, auto_save)
                    }
                    None => tr(strings::CAPTURE_FAILED_CANNOT_READ_THE_SCREEN).to_owned(),
                };
                let _ = tx.send(msg);
                crate::app::request_repaint();
            })
            .ok();
    }

    fn run_overlay(&mut self, ctx: &egui::Context) {
        if self.session.is_none() {
            return;
        }

        if self.waiting_panel {
            if self.restore_frames > 0 {
                self.restore_frames -= 1;
                ctx.request_repaint_after(std::time::Duration::from_millis(25));
                return;
            }
            if !crate::platform::win::is_ready(panel_title()) {
                ctx.request_repaint_after(std::time::Duration::from_millis(25));
                return;
            }
            self.waiting_panel = false;
            crate::platform::util::log("panel restored, creating overlay");
        }
        let mut failed = false;
        let (outcome, cancelled) = {
            let Some(sess) = self.session.as_mut() else {
                return;
            };
            let mon = sess.monitor;

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
            crate::platform::util::log("overlay viewport creation failed, capture aborted");
            self.session = None;
            self.status = tr(strings::COULD_NOT_CREATE_THE_CAPTURE_OVERLAY_TRY_AGAIN).to_owned();
        } else if let Some(o) = outcome {
            self.session = None;
            self.deliver(o);
        } else if cancelled {
            self.session = None;
            self.status = tr(strings::CANCELLED).to_owned();
        }
        if self.session.is_none() && self.remiminimize {
            self.remiminimize = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    fn deliver(&mut self, outcome: Outcome) {
        let img = outcome.image;
        match outcome.action {
            Export::Copy => {
                if self.copy_rx.is_some() {
                    self.status = tr(strings::COPYING_THE_IMAGE).to_owned();
                    return;
                }
                let dir = self.save_dir.clone();
                let auto_save = self.auto_save;
                let (tx, rx) = std::sync::mpsc::channel();
                self.copy_rx = Some(rx);
                self.status = tr(strings::COPYING_THE_IMAGE).to_owned();
                std::thread::Builder::new()
                    .name("rustshot-copy".into())
                    .spawn(move || {
                        let msg = copy_and_save(&img, &dir, auto_save);
                        let _ = tx.send(msg);
                        crate::app::request_repaint();
                    })
                    .ok();
            }
            Export::Save => {
                if self.save_rx.is_some() {
                    self.status = tr(strings::THE_SAVE_DIALOG_IS_ALREADY_OPEN).to_owned();
                    return;
                }
                let dir = self.save_dir.clone();
                let stem = format!("RustShot_{}", crate::platform::util::timestamp());

                let file_name = crate::config::unique_png(&dir, &stem)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "RustShot.png".to_owned());
                let (tx, rx) = std::sync::mpsc::channel();
                self.save_rx = Some(rx);
                self.status = tr(strings::CHOOSING_WHERE_TO_SAVE).to_owned();

                std::thread::Builder::new()
                    .name("rustshot-save".into())
                    .spawn(move || {
                        let picked = rfd::FileDialog::new()
                            .set_directory(&dir)
                            .set_file_name(file_name)
                            .add_filter(tr(strings::PNG_IMAGE), &["png"])
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

    pub(crate) fn persist(&self) {
        crate::config::Config {
            save_dir: self.save_dir.clone(),
            auto_save: self.auto_save,
            theme: self.theme_pref,
            lang: self.lang_pref,
            auto_update: self.auto_update,
            proxy: self.proxy.clone(),
        }
        .save();
    }
}

fn copy_and_save(img: &image::RgbaImage, dir: &std::path::Path, auto_save: bool) -> String {
    let mut msg = match copy_to_clipboard(img) {
        Ok(()) => tr(strings::COPIED_TO_CLIPBOARD).to_owned(),
        Err(e) => tr1(strings::COPY_FAILED, e),
    };
    if auto_save {
        match save_auto_to(dir, img) {
            Ok(p) => msg.push_str(&tr1(strings::SAVED, p.display())),
            Err(e) => msg.push_str(&tr1(strings::AUTO_SAVE_FAILED, e)),
        }
    }
    msg
}

fn save_auto_to(dir: &std::path::Path, img: &image::RgbaImage) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stem = format!("RustShot_{}", crate::platform::util::timestamp());
    let path = crate::config::unique_png(dir, &stem);
    save_png(img, &path)?;
    Ok(path)
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

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_to_tray(ctx);
            self.status = tr(strings::HIDDEN_TO_TRAY_F1_CAPTURES_RIGHT_CLICK_THE_TRAY).to_owned();
            crate::platform::util::log("panel close -> hidden to tray");
        }
        self.poll_save();
        self.poll_copy();
        self.poll_dir();
        self.poll_update();
        if self.reveal_on_update
            && !matches!(self.update, UpdateStatus::Idle | UpdateStatus::Checking)
        {
            self.reveal_on_update = false;
            self.show_panel(ctx);
        }
        self.poll_install(ctx);
        self.pump_worker(ctx);
        self.pump_tray(ctx);
        if self.panel_park.is_some() {
            crate::platform::win::hide_from_taskbar(panel_title());
        }
        if let Some(start) = self.start.take() {
            match start {
                Start::Panel => {}
                Start::Region => self.begin_capture(ctx, Tool::Select, false),
                Start::Full => self.capture_full(),
                Start::Tray => {
                    self.hide_to_tray(ctx);
                    self.status =
                        tr(strings::RUNNING_IN_THE_TRAY_F1_REGION_F2_FULL_SCREEN).to_owned();
                    crate::platform::util::log("started in tray");
                }
                Start::Check => {
                    self.hide_to_tray(ctx);
                    self.reveal_on_update = true;
                    crate::platform::util::log("started to check for updates");
                    self.check_update();
                }
            }
        }
        if self.session.is_some() && !self.paint_hidden_sent {
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
        let c = visuals.panel_fill;
        [
            c.r() as f32 / 255.0,
            c.g() as f32 / 255.0,
            c.b() as f32 / 255.0,
            1.0,
        ]
    }
}
