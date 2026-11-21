use super::*;

impl Session {
    pub(crate) fn show_toolbar(
        &mut self,
        ctx: &egui::Context,
        screen: egui::Rect,
        sel_pts: Option<egui::Rect>,
    ) -> Vec<egui::Rect> {
        let width = self.toolbar_w.max(120.0);
        let x = match sel_pts {
            Some(r) => {
                (r.center().x - width * 0.5).clamp(8.0, (screen.right() - width - 8.0).max(8.0))
            }
            None => (screen.center().x - width * 0.5).max(8.0),
        };
        let h = self.toolbar_h.max(36.0);
        let y = match sel_pts {
            Some(r) if r.bottom() + h + 18.0 < screen.bottom() => r.bottom() + 10.0,
            Some(r) if r.top() - h - 14.0 > screen.top() => r.top() - h - 10.0,
            Some(_) => (screen.bottom() - h - 12.0).max(screen.top() + 4.0),
            None => (screen.bottom() - h - 12.0).max(screen.top() + 4.0),
        };

        let mut export: Option<Export> = None;
        let mut cancel = false;
        let mut undo = false;

        let area = egui::Area::new(egui::Id::unique("rustshot_toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(x, y))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(crate::theme::card())
                    .stroke(egui::Stroke::new(1.0, crate::theme::line()))
                    .corner_radius(9.0)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 14,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(40),
                    })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 4.0);
                        ui.horizontal(|ui| {
                            for tool in Tool::all() {
                                let selected = self.tool == tool;
                                let text = egui::RichText::new(format!(
                                    "{} {}",
                                    tool.icon(),
                                    tool.label()
                                ))
                                .size(13.0);
                                if ui.selectable_label(selected, text).clicked() {
                                    self.tool = tool;
                                }
                            }
                            ui.separator();
                            for w in WIDTHS {
                                let label = if w < 3.0 {
                                    tr(strings::S)
                                } else if w < 6.0 {
                                    tr(strings::M)
                                } else {
                                    tr(strings::L)
                                };
                                if ui
                                    .selectable_label((self.width - w).abs() < 0.01, label)
                                    .clicked()
                                {
                                    self.width = w;
                                }
                            }
                            ui.separator();
                            for c in PALETTE {
                                let (rect, resp) = ui.allocate_exact_size(
                                    egui::vec2(18.0, 18.0),
                                    egui::Sense::click(),
                                );
                                ui.painter().rect_filled(rect, 5.0, c);
                                if self.color == c {
                                    ui.painter().rect_stroke(
                                        rect.expand(2.0),
                                        6.0,
                                        egui::Stroke::new(1.5, crate::theme::text()),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                                if resp.clicked() {
                                    self.color = c;
                                }
                                if resp.hovered() {
                                    ui.painter().rect_stroke(
                                        rect.expand(1.0),
                                        6.0,
                                        egui::Stroke::new(1.0, crate::theme::text_faint()),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }
                            ui.separator();
                            if ui.button(tr(strings::UNDO)).clicked() {
                                undo = true;
                            }
                            if ui.button(tr(strings::COPY)).clicked() {
                                export = Some(Export::Copy);
                            }
                            if ui.button(tr(strings::SAVE)).clicked() {
                                export = Some(Export::Save);
                            }
                            if ui.button(tr(strings::CANCEL)).clicked() {
                                cancel = true;
                            }
                        });
                    });
            });

        if undo {
            self.undo();
        }
        if let Some(a) = export {
            self.export(a);
        }
        if cancel {
            self.cancelled = true;
        }

        self.toolbar_w = area.response.rect.width();
        self.toolbar_h = area.response.rect.height();
        vec![area.response.rect.expand(4.0)]
    }
}
