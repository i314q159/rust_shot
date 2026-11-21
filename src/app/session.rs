use super::*;
use crate::app::files::{copy_and_save, save_png};

impl App {
    pub(crate) fn run_overlay(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn deliver(&mut self, outcome: Outcome) {
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
