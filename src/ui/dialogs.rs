use crate::app::{APP_AUTHOR, APP_VERSION, App, Install, LICENSE_TEXT};
use crate::config::i18n::strings;
use crate::config::i18n::{tr, tr1, tr2};
use crate::update::{self, Status as UpdateStatus};

impl App {
    pub(crate) fn proxy_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_proxy {
            return;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new(tr(strings::PROXY_3))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(tr(strings::ONLY_USED_BY_THE_UPDATE_CHECK_AND_DOWNLOAD))
                        .size(11.0)
                        .color(crate::theme::text_faint()),
                );
                ui.add_space(6.0);
                let before = self.proxy_mode;
                let mut mode = self.proxy_mode;
                ui.radio_value(&mut mode, 0, tr(strings::SYSTEM_PROXY_FOLLOW_WINDOWS));
                ui.radio_value(&mut mode, 1, tr(strings::DIRECT_NO_PROXY_2));
                ui.radio_value(&mut mode, 2, tr(strings::MANUAL_ADDRESS));
                if self.proxy_mode == 2 {
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.label(tr(strings::PROXY_IP));
                        let host_edit = ui.add(
                            egui::TextEdit::singleline(&mut self.proxy_host)
                                .hint_text("127.0.0.1")
                                .desired_width(110.0),
                        );
                        ui.label(tr(strings::PROXY_PORT));
                        let port_edit = ui.add(
                            egui::TextEdit::singleline(&mut self.proxy_port)
                                .hint_text("12334")
                                .desired_width(56.0),
                        );

                        let valid = self.manual_proxy().is_some();
                        if host_edit.changed() || port_edit.changed() {
                            self.proxy_bad = !valid;
                        }
                        let can_save = !self.proxy_bad && valid;
                        let enter = (host_edit.lost_focus() || port_edit.lost_focus())
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui
                            .add_enabled(can_save, egui::Button::new(tr(strings::SAVE)))
                            .clicked()
                            || (can_save && enter)
                        {
                            self.apply_proxy(2);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(22.0);
                        ui.label(
                            egui::RichText::new(tr(
                                strings::TYPE_IT_YOURSELF_ENTER_OR_SAVE_WRITES_CONFIG,
                            ))
                            .size(10.5)
                            .color(crate::theme::text_faint()),
                        );
                        if self.proxy_bad {
                            ui.label(
                                egui::RichText::new(tr(strings::INVALID_ADDRESS))
                                    .size(10.5)
                                    .color(crate::theme::warn()),
                            );
                        }
                    });
                }
                if mode != before {
                    self.apply_proxy(mode);
                }
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(tr1(strings::NOW, self.proxy.label()))
                            .size(11.0)
                            .color(crate::theme::text_weak()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(strings::CLOSE)).clicked() {
                            close = true;
                        }
                    });
                });
            });
        if close {
            open = false;
        }
        if !open {
            self.show_proxy = false;
        }
    }

    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        self.update_dialog(ctx);
        self.proxy_dialog(ctx);
        self.license_dialog(ctx);
    }

    fn license_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_license {
            return;
        }
        let mut open = true;
        egui::Window::new(tr(strings::LICENSE_MIT))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 420.0])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(tr2(strings::LICENSE_CREDIT, APP_VERSION, APP_AUTHOR))
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
                    if ui.button(tr(strings::COPY_FULL_TEXT)).clicked() {
                        ui.ctx().copy_text(LICENSE_TEXT.to_owned());
                    }
                    ui.label(
                        egui::RichText::new(tr(strings::FULL_TEXT_LICENSE_MIT_MD))
                            .size(11.0)
                            .color(crate::theme::text_faint()),
                    );
                });
            });
        if !open {
            self.show_license = false;
        }
    }

    pub(crate) fn update_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_update {
            return;
        }
        let mut open = true;
        let mut retry = false;
        let mut close = false;
        let mut install_now = false;
        let mut silent_toggle: Option<bool> = None;
        let mut page: Option<String> = None;
        let current = self.update.clone();
        let release_url = match &current {
            UpdateStatus::Newer(rel) => rel.url.clone(),
            _ => update::RELEASES_URL.to_owned(),
        };
        egui::Window::new(tr(strings::CHECK_FOR_UPDATES))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(tr2(strings::UPDATE_CREDIT, APP_VERSION, update::REPO))
                        .size(11.0)
                        .color(crate::theme::text_faint()),
                );
                let mut silent = self.auto_update == crate::update::AutoUpdate::Silent;
                if ui
                    .checkbox(
                        &mut silent,
                        tr(strings::INSTALL_NEW_VERSIONS_SILENTLY_DO_NOT_ASK),
                    )
                    .on_hover_text(tr(
                        strings::WRITES_AUTO_UPDATE_SILENT_TO_CONFIG_TOML_ONE_UAC,
                    ))
                    .changed()
                {
                    silent_toggle = Some(silent);
                }
                ui.add_space(6.0);
                match &current {
                    UpdateStatus::Idle => {
                        ui.label(egui::RichText::new(tr(strings::NOT_CHECKED_YET)).size(12.5));
                    }
                    UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                egui::RichText::new(tr(strings::CONNECTING_TO_GITHUB)).size(12.5),
                            );
                        });
                    }
                    UpdateStatus::Latest(version) => {
                        ui.label(
                            egui::RichText::new(tr1(strings::UP_TO_DATE_GITHUB_LATEST_V, version))
                                .size(13.0)
                                .color(crate::theme::ok()),
                        );
                        ui.label(
                            egui::RichText::new(tr(strings::NEW_RELEASES_CAN_BE_OPENED_FROM_HERE))
                                .size(11.5)
                                .color(crate::theme::text_weak()),
                        );
                    }
                    UpdateStatus::Newer(rel) => {
                        ui.label(
                            egui::RichText::new(tr1(strings::NEW_VERSION_V, &rel.version))
                                .size(15.0)
                                .strong()
                                .color(crate::theme::ok()),
                        );
                        let mut meta = rel.name.clone();
                        if !rel.published.is_empty() {
                            if !meta.trim().is_empty() {
                                meta.push_str(" · ");
                            }
                            meta.push_str(tr(strings::RELEASED));
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
                                egui::RichText::new(tr(strings::THIS_RELEASE_HAS_NO_NOTES))
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
                            egui::RichText::new(tr(strings::UPDATE_CHECK_FAILED_2))
                                .size(13.0)
                                .color(crate::theme::warn()),
                        );
                        ui.label(
                            egui::RichText::new(err)
                                .size(11.5)
                                .color(crate::theme::text_weak()),
                        );
                        ui.label(
                            egui::RichText::new(tr(strings::YOU_CAN_ALSO_OPEN_THE_RELEASES_PAGE))
                                .size(11.0)
                                .color(crate::theme::text_faint()),
                        );
                    }
                }
                if matches!(current, UpdateStatus::Newer(_)) {
                    ui.add_space(6.0);
                    match &self.install {
                        Install::Idle => {
                            ui.label(
                                egui::RichText::new(tr(
                                    strings::DOWNLOAD_VERIFY_AND_INSTALL_SILENTLY_ONE_UAC,
                                ))
                                .size(11.0)
                                .color(crate::theme::text_faint()),
                            );
                        }
                        Install::Downloading { got, total } => {
                            let pct = got
                                .saturating_mul(100)
                                .checked_div(*total)
                                .map(|p| p.min(100));
                            let ratio = pct.map(|p| p as f32 / 100.0).unwrap_or(0.0);
                            let text = match pct {
                                Some(p) => format!(
                                    "{}  ({} / {})",
                                    tr1(strings::DOWNLOADING_2, p),
                                    update::human_size(*got),
                                    update::human_size(*total)
                                ),
                                None => tr1(strings::DOWNLOADING, update::human_size(*got)),
                            };
                            ui.add(
                                egui::ProgressBar::new(ratio)
                                    .desired_width(400.0)
                                    .text(text),
                            );
                        }
                        Install::Verifying => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(
                                    egui::RichText::new(tr(strings::VERIFYING_SHA_256)).size(12.0),
                                );
                            });
                        }
                        Install::Launching => {
                            ui.label(
                                egui::RichText::new(tr(
                                    strings::VERIFIED_INSTALLING_AND_RESTARTING,
                                ))
                                .size(12.0)
                                .color(crate::theme::ok()),
                            );
                        }
                        Install::Failed(err) => {
                            ui.label(
                                egui::RichText::new(tr(strings::UPDATE_FAILED_2))
                                    .size(12.5)
                                    .color(crate::theme::warn()),
                            );
                            ui.label(
                                egui::RichText::new(err)
                                    .size(11.0)
                                    .color(crate::theme::text_weak()),
                            );
                        }
                    }
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let busy = matches!(current, UpdateStatus::Checking);
                    let installing = matches!(
                        self.install,
                        Install::Downloading { .. } | Install::Verifying | Install::Launching
                    );
                    if matches!(current, UpdateStatus::Newer(_)) && !crate::update::installed_copy()
                    {
                        ui.label(
                            egui::RichText::new(tr(
                                strings::PORTABLE_BUILDS_DO_NOT_SELF_UPDATE_DOWNLOAD_THE_2,
                            ))
                            .size(11.0)
                            .color(crate::theme::text_faint()),
                        );
                    }
                    if matches!(current, UpdateStatus::Newer(_))
                        && crate::update::installed_copy()
                        && !installing
                        && ui
                            .add_enabled(
                                !busy,
                                egui::Button::new(if matches!(self.install, Install::Failed(_)) {
                                    tr(strings::RETRY_UPDATE)
                                } else {
                                    tr(strings::UPDATE_NOW)
                                }),
                            )
                            .clicked()
                    {
                        install_now = true;
                    }
                    if ui
                        .add_enabled(!busy, egui::Button::new(tr(strings::CHECK_AGAIN)))
                        .clicked()
                    {
                        retry = true;
                    }
                    if ui.button(tr(strings::OPEN_RELEASES_PAGE)).clicked() {
                        page = Some(release_url.clone());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr(strings::CLOSE)).clicked() {
                            close = true;
                        }
                    });
                });
            });
        if let Some(silent) = silent_toggle {
            self.auto_update = if silent {
                crate::update::AutoUpdate::Silent
            } else {
                crate::update::AutoUpdate::Notice
            };
            self.persist();
        }
        if install_now {
            self.install = Install::Idle;
            self.start_install();
        }
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
}
