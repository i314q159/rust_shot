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

mod files;
mod poll;
mod session;
mod theme;
mod window;
