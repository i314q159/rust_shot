use super::*;

impl Session {
    pub(crate) fn start_text(&mut self, p: Px) {
        let size = (self.width * 6.5).clamp(14.0, 64.0);
        self.shapes.push(Shape::Text {
            pos: p,
            text: String::new(),
            color: self.color,
            size,
        });
        self.text_idx = Some(self.shapes.len() - 1);
        self.text_focus_done = false;
    }

    pub(crate) fn commit_text(&mut self) {
        let Some(idx) = self.text_idx.take() else {
            return;
        };
        self.text_focus_done = false;
        let Some(Shape::Text { text, .. }) = self.shapes.get_mut(idx) else {
            return;
        };
        while text.ends_with('\n') || text.ends_with('\r') {
            text.pop();
        }
        if text.trim().is_empty() {
            self.shapes.remove(idx);
        }
    }

    pub(crate) fn cancel_text(&mut self) {
        let Some(idx) = self.text_idx.take() else {
            return;
        };
        self.text_focus_done = false;
        if idx < self.shapes.len() {
            self.shapes.remove(idx);
        }
    }

    pub(crate) fn show_text_editor(
        &mut self,
        ctx: &egui::Context,
        rx: f32,
        ry: f32,
    ) -> Vec<egui::Rect> {
        let Some(idx) = self.text_idx else {
            return Vec::new();
        };
        let screen = ctx.viewport_rect();
        let (pos, _) = match self.shapes.get(idx) {
            Some(Shape::Text { pos, size, .. }) => (
                egui::pos2(screen.min.x + pos.0 / rx, screen.min.y + pos.1 / ry),
                *size,
            ),
            _ => return Vec::new(),
        };

        let at = egui::pos2(
            (pos.x - 4.0)
                .min(screen.right() - 244.0)
                .max(screen.left() + 4.0),
            (pos.y - 4.0)
                .min(screen.bottom() - 48.0)
                .max(screen.top() + 4.0),
        );

        let mut rects = Vec::new();
        let want_focus = !self.text_focus_done;
        let shapes = &mut self.shapes;

        let editing = match shapes.get(idx) {
            Some(Shape::Text { color, .. }) => *color,
            _ => crate::theme::text(),
        };
        let box_fill = if is_bright(editing) {
            egui::Color32::from_black_alpha(200)
        } else {
            egui::Color32::from_white_alpha(235)
        };
        let area = egui::Area::new(egui::Id::unique("rustshot_text_editor"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(box_fill)
                    .stroke(egui::Stroke::new(1.0, crate::theme::accent()))
                    .corner_radius(3.0)
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        if let Some(Shape::Text {
                            text, color, size, ..
                        }) = shapes.get_mut(idx)
                        {
                            let te = ui.add(
                                egui::TextEdit::multiline(text)
                                    .font(egui::FontId::proportional(*size / ((rx + ry) * 0.5)))
                                    .text_color(*color)
                                    .desired_width(220.0)
                                    .desired_rows(1)
                                    .frame(egui::Frame::NONE),
                            );
                            if want_focus {
                                te.request_focus();
                            }
                        }
                    });
            });
        if want_focus {
            self.text_focus_done = true;
        }
        rects.push(area.response.rect.expand(6.0));
        rects
    }
}
