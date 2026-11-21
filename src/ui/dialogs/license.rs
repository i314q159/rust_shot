use super::*;

impl App {
    pub(crate) fn license_dialog(&mut self, ctx: &egui::Context) {
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
}
