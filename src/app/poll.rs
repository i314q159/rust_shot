use super::*;

impl App {
    pub(crate) fn poll_install(&mut self, ctx: &egui::Context) {
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

    pub(crate) fn quit_for_update(&mut self, ctx: &egui::Context) {
        crate::platform::util::log("update: verified, quitting so the helper can install");
        crate::app::worker::release_instance();
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    pub(crate) fn poll_update(&mut self) {
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

    pub(crate) fn poll_dir(&mut self) {
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

    pub(crate) fn poll_copy(&mut self) {
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

    pub(crate) fn poll_save(&mut self) {
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
}
