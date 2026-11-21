

use crate::app::{APP_VERSION, App};
use crate::capture::overlay::Tool;
use crate::config::i18n::strings;
use crate::config::i18n::{tr, tr1};
use crate::update::{self, Status as UpdateStatus};

impl App {
    pub(crate) fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(crate::theme::bar())
            .inner_margin(egui::Margin::symmetric(10, 3))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.menu_button(tr(strings::CAPTURE), |ui| {
                        if ui.button(tr(strings::REGION_CAPTURE_F1)).clicked() {
                            let ctx = ui.ctx().clone();
                            self.begin_capture(&ctx, Tool::Select, false);
                            ui.close();
                        }
                        if ui.button(tr(strings::FULL_SCREEN_F2)).clicked() {
                            let ctx = ui.ctx().clone();
                            self.begin_capture(&ctx, Tool::Select, true);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(tr(strings::OPEN_SAVE_FOLDER)).clicked() {
                            let _ = std::fs::create_dir_all(&self.save_dir);
                            let _ = std::process::Command::new("explorer")
                                .arg(&self.save_dir)
                                .spawn();
                            ui.close();
                        }
                    });
                    ui.menu_button(tr(strings::THEME_2), |ui| {
                        let system_dark = crate::theme::system_is_dark();
                        for pref in [
                            crate::theme::Preference::System,
                            crate::theme::Preference::Light,
                            crate::theme::Preference::Dark,
                        ] {
                            let label = match pref {
                                crate::theme::Preference::System => tr1(
                                    strings::FOLLOW_SYSTEM_NOW,
                                    if system_dark {
                                        tr(strings::DARK_2)
                                    } else {
                                        tr(strings::LIGHT_2)
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
                    ui.menu_button(tr(strings::LANGUAGE_2), |ui| {
                        let system = crate::config::i18n::detect();
                        for pref in [
                            crate::config::i18n::Preference::System,
                            crate::config::i18n::Preference::Zh,
                            crate::config::i18n::Preference::En,
                        ] {
                            let label = match pref {
                                crate::config::i18n::Preference::System => {
                                    tr1(strings::FOLLOW_SYSTEM_NOW, system.label())
                                }
                                other => other.label(),
                            };
                            if ui.selectable_label(self.lang_pref == pref, label).clicked() {
                                let ctx = ui.ctx().clone();
                                self.set_lang(&ctx, pref);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button(tr(strings::HELP), |ui| {
                        if ui.button(tr(strings::CHECK_FOR_UPDATES)).clicked() {
                            self.check_update();
                            ui.close();
                        }
                        if ui.button(tr(strings::OPEN_RELEASES_PAGE_GITHUB)).clicked() {
                            update::open_page(update::RELEASES_URL);
                            ui.close();
                        }
                        ui.separator();
                        if ui
                            .button(tr1(strings::PROXY_NOW, self.proxy.label()))
                            .clicked()
                        {
                            self.show_proxy = true;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button(tr(strings::LICENSE_MIT_2)).clicked() {
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
                                    egui::RichText::new(tr1(strings::UPDATE_V, rel.version))
                                        .size(10.5)
                                        .color(crate::theme::accent()),
                                )
                                .frame(false);
                                if ui
                                    .add(badge)
                                    .on_hover_text(tr(strings::CLICK_TO_SEE_WHAT_S_NEW))
                                    .clicked()
                                {
                                    self.show_update = true;
                                }
                            }
                            UpdateStatus::Checking => {
                                ui.label(
                                    egui::RichText::new(tr(strings::CHECKING))
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

pub(crate) fn big_button(ui: &mut egui::Ui, title: &str, hint: &str, width: f32) -> egui::Response {
    let text = egui::RichText::new(format!("{title}\n{hint}")).size(13.0);
    ui.add_sized([width, 58.0], egui::Button::new(text).corner_radius(9.0))
}
