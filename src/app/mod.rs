//! eframe application: control panel, overlay orchestration, clipboard / file plumbing.

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

/// Window title of the control panel (also used to find the HWND).
pub fn panel_title() -> &'static str {
    crate::config::i18n::tr(strings::RUSTSHOT_SCREENSHOT)
}

/// Drawn icon assets (see `assets/icon/preview.png`).
/// The licence text shipped with the binary and shown in the Help menu.
pub(crate) const LICENSE_TEXT: &str = include_str!("../../LICENSE-MIT.md");
pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const APP_AUTHOR: &str = "i314q159";

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
    /// Tray, then immediately check for updates (`--check-update`).
    Check,
}

/// Progress of the silent update (`立即更新` in the update dialog).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Install {
    Idle,
    Downloading { got: u64, total: u64 },
    Verifying,
    Launching,
    Failed(String),
}

/// Messages from the download/verify thread.
pub(crate) enum InstallEvent {
    Progress(u64, u64),
    Verifying,
    Failed(String),
    Done,
}

/// Result of the (worker thread) save dialog.
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
    /// True only when the user really asked to quit (tray menu / exit button).
    pub(crate) quitting: bool,
    /// Set while the panel is being restored so the overlay can be created.
    pub(crate) waiting_panel: bool,
    /// Frames still to wait before the restored panel may host a viewport.
    pub(crate) restore_frames: u8,
    /// Bumped per capture so every overlay gets a fresh viewport id.
    pub(crate) serial: u64,
    /// Put the panel back into the tray once the capture is finished.
    pub(crate) remiminimize: bool,
    /// Where the panel lives while it is parked off screen (hidden to the tray).
    pub(crate) panel_park: Option<crate::platform::win::Parked>,
    /// RequestPaintWhileHidden only needs to be sent once per capture.
    pub(crate) paint_hidden_sent: bool,
    /// Set while the save dialog runs on its own thread.
    pub(crate) save_rx: Option<std::sync::mpsc::Receiver<SaveResult>>,
    /// Set while the "pick folder" dialog runs on its own thread.
    pub(crate) dir_rx: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
    /// Tray menu clicks arrive on the OS side; queue them here.
    pub(crate) menu_events: Arc<Mutex<Vec<muda::MenuId>>>,
    /// Tray icon clicks arrive on the OS side; queue them here.
    pub(crate) tray_events: Arc<Mutex<Vec<tray_icon::TrayIconEvent>>>,
    /// Help → 开源声明 window is open.
    pub(crate) show_license: bool,
    /// Top bar theme choice (system / light / dark), persisted in config.toml.
    pub(crate) theme_pref: crate::theme::Preference,
    /// Top bar language choice (system / zh / en), persisted in config.toml.
    pub(crate) lang_pref: crate::config::i18n::Preference,
    /// Latest GitHub update check result shown in the panel / dialog.
    pub(crate) update: UpdateStatus,
    /// Set while the update request runs on its own thread.
    pub(crate) update_rx: Option<std::sync::mpsc::Receiver<UpdateStatus>>,
    /// Update dialog is open.
    pub(crate) show_update: bool,
    /// `auto_update=notice|silent`, persisted in config.toml.
    pub(crate) auto_update: crate::update::AutoUpdate,
    /// `proxy=` from config.toml; kept as loaded so saving other keys keeps it.
    pub(crate) proxy: crate::config::proxy::Proxy,
    /// IP / host being edited in the proxy dialog when the proxy is manual.
    pub(crate) proxy_host: String,
    /// Port being edited in the proxy dialog when the proxy is manual.
    pub(crate) proxy_port: String,
    /// Dialog selection: 0 system, 1 direct, 2 manual (kept even while the
    /// typed address is still invalid, otherwise the input boxes would vanish).
    pub(crate) proxy_mode: u8,
    /// The manual IP / port currently typed cannot be parsed.
    pub(crate) proxy_bad: bool,
    /// `帮助 → 网络代理` window is open.
    pub(crate) show_proxy: bool,
    /// Download / verify progress of the silent update.
    pub(crate) install: Install,
    /// Set while the update download runs on its own thread.
    pub(crate) install_rx: Option<std::sync::mpsc::Receiver<InstallEvent>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, start: Start) -> Self {
        let _ = UI_CTX.set(cc.egui_ctx.clone());
        crate::config::fonts::install_egui_font(&cc.egui_ctx);
        let cfg = crate::config::Config::load();
        // First run: drop a commented `config.toml` so the settings are easy to
        // find and edit. Does nothing once the file exists.
        if !crate::config::exists() {
            cfg.save();
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
            // 常驻滚动条：默认的悬浮细条平时几乎看不见，面板压小时会以为内容丢了。
            style.spacing.scroll.floating = false;
            style.spacing.scroll.foreground_color = true;
            style.spacing.scroll.bar_width = 9.0;
            style.spacing.scroll.bar_inner_margin = 3.0;
        });

        let worker = Worker::spawn();
        let (tray, menu) = build_tray();
        crate::platform::util::log(&format!("app created, tray={}", tray.is_some()));

        // Tray start (plain launch / autostart): move the window off screen
        // before the first frame, so nothing flashes on the desktop.
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
        }
    }

    /// Kick off (or restart) the GitHub update check.
    pub(crate) fn check_update(&mut self) {
        if self.update_rx.is_some() {
            return;
        }
        self.update = UpdateStatus::Checking;
        self.update_rx = Some(update::check_async());
        self.show_update = true;
        self.status = tr(strings::CHECKING_FOR_UPDATES).to_owned();
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
        crate::platform::util::log(&format!("theme -> {}", if dark { "dark" } else { "light" }));
        ctx.request_repaint();
    }

    /// Top bar choice: remember it, restyle immediately, write it to config.toml.
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

    /// Top bar language choice: retitle the window and the tray, then save it.
    pub(crate) fn set_lang(&mut self, ctx: &egui::Context, pref: crate::config::i18n::Preference) {
        self.lang_pref = pref;
        crate::config::i18n::set(crate::config::i18n::resolve(pref));
        self.persist();
        // The tray menu lives on the OS side, so its labels have to be pushed.
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

    /// Build a manual proxy from the IP / port boxes, if both are valid.
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

    /// Commit the proxy picked in the dialog (`0` system, `1` direct, `2` manual)
    /// and write it to config.toml. The next check picks it up.
    pub(crate) fn apply_proxy(&mut self, mode: u8) {
        self.proxy_mode = mode;
        match mode {
            1 => {
                self.proxy = crate::config::proxy::Proxy::Direct;
                self.proxy_bad = false;
            }
            2 => {
                // Keep "manual" selected even when a box is empty or garbage;
                // only a valid IP + port is written to config.toml.
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
        if !crate::update::installed_copy() {
            self.status = tr(strings::PORTABLE_BUILDS_DO_NOT_SELF_UPDATE_DOWNLOAD_THE).to_owned();
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
                // Only an installed copy can replace itself; the portable exe
                // updates by downloading a new portable zip from the release.
                let asset = crate::update::setup_name(&rel.version);
                let dest = crate::update::update_dir().join(&asset);
                let result = (|| -> Result<(), String> {
                    let expected = crate::update::expected_sha(&rel.version, &asset)?;
                    let url = crate::update::asset_url(&rel.version, &asset);
                    let _ = tx.send(InstallEvent::Progress(0, 0));
                    // One retry: a flaky proxy or a dropped connection should not
                    // force the user to click again.
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

    /// Release the single-instance mutex and quit so the helper can install.
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

    /// Folder picker on its own thread: `pick_folder` also owns a modal loop.
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
                WorkerEvent::CheckUpdate => self.check_update(),
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
            // No usable window: fall back to a plain minimize.
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

    /// Freeze the monitor under the pointer and open the annotation overlay.
    ///
    /// `full_frame` preselects the whole screen (F2): Enter copies it as-is, the
    /// tools annotate on top, and a drag narrows it down to a region.
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
                // eframe cannot create child viewports from a hidden or
                // minimized window, so the panel has to be mapped before the
                // overlay appears. When it is parked off screen that already
                // holds and nothing visible happens; only a panel the user
                // minimized by hand has to be restored for a frame.
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

    /// --full from the command line: freeze, render the whole monitor and copy
    /// it, without opening the overlay (scripts have nothing to click).
    pub(crate) fn capture_full(&mut self) {
        let monitor = capture::monitor_at_cursor();
        match capture::capture(monitor) {
            Some(frame) => {
                let rect = PhysRect::new(0, 0, frame.w as i32, frame.h as i32);
                let img = crate::capture::shapes::render(&frame, rect, &[]);
                self.deliver(Outcome {
                    action: Export::Copy,
                    image: img,
                });
            }
            None => self.status = tr(strings::CAPTURE_FAILED_CANNOT_READ_THE_SCREEN).to_owned(),
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
                    Ok(()) => tr(strings::COPIED_TO_CLIPBOARD).to_owned(),
                    Err(e) => tr1(strings::COPY_FAILED, e),
                };
                if self.auto_save {
                    match self.save_auto(&img) {
                        Ok(p) => msg.push_str(&tr1(strings::SAVED, p.display())),
                        Err(e) => msg.push_str(&tr1(strings::AUTO_SAVE_FAILED, e)),
                    }
                }
                self.status = msg;
            }
            Export::Save => {
                if self.save_rx.is_some() {
                    self.status = tr(strings::THE_SAVE_DIALOG_IS_ALREADY_OPEN).to_owned();
                    return;
                }
                let dir = self.save_dir.clone();
                let stem = format!("RustShot_{}", crate::platform::util::timestamp());
                // `set_file_name` wants a bare file name, not a full path.
                let file_name = crate::config::unique_png(&dir, &stem)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "RustShot.png".to_owned());
                let (tx, rx) = std::sync::mpsc::channel();
                self.save_rx = Some(rx);
                self.status = tr(strings::CHOOSING_WHERE_TO_SAVE).to_owned();
                // The dialog owns a nested message loop and a COM apartment:
                // running it inside our paint pass froze the window, and it also
                // ended up hidden behind the always-on-top overlay.
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

    fn save_auto(&self, img: &image::RgbaImage) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.save_dir).map_err(|e| e.to_string())?;
        let stem = format!("RustShot_{}", crate::platform::util::timestamp());
        let path = crate::config::unique_png(&self.save_dir, &stem);
        save_png(img, &path)?;
        Ok(path)
    }
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
    /// Runs even while the window is hidden/minimized, so hotkeys keep working.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Closing the panel window goes to the tray instead of quitting.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_to_tray(ctx);
            self.status = tr(strings::HIDDEN_TO_TRAY_F1_CAPTURES_RIGHT_CLICK_THE_TRAY).to_owned();
            crate::platform::util::log("panel close -> hidden to tray");
        }
        self.poll_save();
        self.poll_dir();
        self.poll_update();
        self.poll_install(ctx);
        self.pump_worker(ctx);
        self.pump_tray(ctx);
        if self.panel_park.is_some() {
            // eframe re-applies winit's window styles on the first frames, which
            // would put the parked panel back into the taskbar.
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
                    crate::platform::util::log("started to check for updates");
                    self.check_update();
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
