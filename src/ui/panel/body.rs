use super::*;

impl App {
    pub(crate) fn panel(&mut self, ui: &mut egui::Ui) {
        self.menu_bar(ui);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(crate::theme::bg())
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ui, |ui| {
                let footer_h = 42.0;
                let body_h = (ui.available_height() - footer_h).max(80.0);
                egui::ScrollArea::vertical()
                    .id_salt("panel_body")
                    .max_height(body_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(egui::RichText::new("RustShot").size(24.0).strong());
                            ui.label(
                                egui::RichText::new(tr(strings::SCREENSHOT_ANNOTATE))
                                    .size(12.0)
                                    .color(crate::theme::text_faint()),
                            );
                        });
                        ui.add_space(10.0);

                        let bw = (ui.available_width() - 16.0) / 2.0;
                        ui.horizontal(|ui| {
                            if big_button(ui, tr(strings::REGION_CAPTURE), "F1", bw).clicked() {
                                let ctx = ui.ctx().clone();
                                self.begin_capture(&ctx, Tool::Select, false);
                            }
                            if big_button(ui, tr(strings::FULL_SCREEN), "F2", bw).clicked() {
                                let ctx = ui.ctx().clone();
                                self.begin_capture(&ctx, Tool::Select, true);
                            }
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(
                            egui::RichText::new(tr(strings::SAVE_FOLDER_2))
                                .size(12.5)
                                .color(crate::theme::text_weak()),
                        )
                        .on_hover_text(match crate::config::path() {
                            Some(p) => tr1(
                                strings::CONFIG_EDIT_SAVE_DIR_AUTO_SAVE_THEME_LANG_BY,
                                p.display(),
                            ),
                            None => {
                                tr(strings::CONFIG_LIVES_IN_CONFIG_TOML_NEXT_TO_THE_EXE).to_owned()
                            }
                        });
                        ui.horizontal(|ui| {
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
                            if ui.small_button(tr(strings::CHANGE)).clicked() {
                                self.pick_dir();
                            }
                            if ui.small_button(tr(strings::OPEN)).clicked() {
                                let _ = std::fs::create_dir_all(&self.save_dir);
                                let _ = std::process::Command::new("explorer")
                                    .arg(&self.save_dir)
                                    .spawn();
                            }
                            if ui
                                .small_button(tr(strings::RESET))
                                .on_hover_text(tr(
                                    strings::RESTORE_THE_DEFAULT_FOLDER_AND_WRITE_IT_TO,
                                ))
                                .clicked()
                            {
                                self.save_dir = crate::platform::util::default_save_dir();
                                self.persist();
                                self.status = tr1(
                                    strings::RESET_TO_THE_DEFAULT_FOLDER,
                                    self.save_dir.display(),
                                );
                            }
                        });
                        if ui
                            .checkbox(
                                &mut self.auto_save,
                                tr(strings::ALSO_SAVE_A_PNG_WHEN_COPYING),
                            )
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
                                for text in strings::TIPS {
                                    ui.label(
                                        egui::RichText::new(tr(text))
                                            .size(11.5)
                                            .color(crate::theme::text_weak()),
                                    );
                                }
                            });

                        ui.add_space(8.0);
                        if !self.status.is_empty() {
                            ui.label(
                                egui::RichText::new(&self.status)
                                    .size(12.0)
                                    .color(crate::theme::ok()),
                            );
                        }
                    });

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        if self.tray.is_none() {
                            ui.label(
                                egui::RichText::new(tr(strings::TRAY_UNAVAILABLE))
                                    .size(11.0)
                                    .color(crate::theme::warn()),
                            );
                        }
                        ui.label(
                            egui::RichText::new(tr1(
                                strings::FONT,
                                crate::config::fonts::cjk()
                                    .map(|f| f.label.clone())
                                    .unwrap_or_else(|| tr(strings::SYSTEM_DEFAULT).into()),
                            ))
                            .size(10.5)
                            .color(crate::theme::text_faint()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(tr(strings::QUIT)).clicked() {
                                self.quitting = true;
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui.button(tr(strings::HIDE_TO_TRAY)).clicked() {
                                let ctx = ui.ctx().clone();
                                self.hide_to_tray(&ctx);
                            }
                        });
                    });
                });
            });
    }
}
