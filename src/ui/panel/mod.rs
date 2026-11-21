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
}

pub(crate) fn big_button(ui: &mut egui::Ui, title: &str, hint: &str, width: f32) -> egui::Response {
    let text = egui::RichText::new(format!("{title}\n{hint}")).size(13.0);
    ui.add_sized([width, 58.0], egui::Button::new(text).corner_radius(9.0))
}

mod body;
