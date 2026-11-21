use super::*;
use crate::app::files::copy_and_save;

impl App {
    pub(crate) fn show_panel(&mut self, ctx: &egui::Context) {
        if let Some(parked) = self.panel_park.take() {
            crate::platform::win::unpark(panel_title(), parked);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    pub(crate) fn pump_worker(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn pump_tray(&mut self, ctx: &egui::Context) {
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
}
