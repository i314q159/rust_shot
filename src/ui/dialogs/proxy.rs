use super::*;

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
}
