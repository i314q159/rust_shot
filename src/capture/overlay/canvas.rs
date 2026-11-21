use super::*;

impl Session {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let screen = ctx.viewport_rect();
        if self.fix_tries == 0 {
            crate::platform::util::log(&format!(
                "overlay frame0 ppp={:.4} screen={screen:?} frame={}x{} monitor={:?}",
                ctx.pixels_per_point(),
                self.frame.w,
                self.frame.h,
                self.monitor
            ));
        }

        if self.fix_tries < 8 {
            let ok = crate::platform::win::force_rect_by_title(&self.title, self.monitor);
            self.fix_tries += 1;
            if ok && self.fix_tries > 2 {
                crate::platform::util::log("overlay geometry pinned");
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.fix_tries = 8;
            }
            ctx.request_repaint();
        }
        if screen.width() < 8.0 || screen.height() < 8.0 {
            ctx.request_repaint();
            return;
        }

        self.rx = self.frame.w as f32 / screen.width();
        self.ry = self.frame.h as f32 / screen.height();
        let (rx, ry) = (self.rx, self.ry);
        let origin = screen.min;

        if self.tex.is_none() {
            self.tex = Some(ctx.load_texture(
                "rustshot_frozen",
                self.frame.to_color_image(),
                egui::TextureOptions::NEAREST,
            ));
        }

        let painter = ui.painter_at(screen);
        if let Some(tex) = &self.tex {
            painter.image(
                tex.id(),
                screen,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        let to_p = |p: egui::Pos2| egui::pos2(origin.x + p.x / rx, origin.y + p.y / ry);
        let to_phys = |p: egui::Pos2| egui::pos2((p.x - origin.x) * rx, (p.y - origin.y) * ry);

        let dim = egui::Color32::from_black_alpha(64);
        match self.sel {
            Some(sel) => {
                let r = egui::Rect::from_min_max(to_p(sel.min), to_p(sel.max));
                let s = screen;
                if r.top() > s.top() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(s.min, egui::pos2(s.right(), r.top())),
                        0.0,
                        dim,
                    );
                }
                if r.bottom() < s.bottom() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(egui::pos2(s.left(), r.bottom()), s.max),
                        0.0,
                        dim,
                    );
                }
                if r.left() > s.left() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(s.left(), r.top()),
                            egui::pos2(r.left(), r.bottom()),
                        ),
                        0.0,
                        dim,
                    );
                }
                if r.right() < s.right() {
                    painter.rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(r.right(), r.top()),
                            egui::pos2(s.right(), r.bottom()),
                        ),
                        0.0,
                        dim,
                    );
                }
            }
            None => {
                painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha(52));
                if let Some(h) = self.hover {
                    let r = egui::Rect::from_min_max(to_p(h.min), to_p(h.max));
                    painter.rect_stroke(
                        r,
                        0.0,
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(70, 160, 255)),
                        egui::StrokeKind::Outside,
                    );
                    self.size_tag(&painter, r, h.width() as u32, h.height() as u32);
                }
            }
        }

        if let Some(sel) = self.sel {
            let clip = egui::Rect::from_min_max(to_p(sel.min), to_p(sel.max));
            let sp = painter.with_clip_rect(clip);
            for s in &self.shapes {
                shapes::paint(&sp, s, rx, ry, origin);
            }
        }

        if let Some(sel) = self.sel {
            let r = egui::Rect::from_min_max(to_p(sel.min), to_p(sel.max));
            painter.rect_stroke(
                r,
                0.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(64, 158, 255)),
                egui::StrokeKind::Outside,
            );

            let c = r.center();
            let pts = [
                r.left_top(),
                egui::pos2(c.x, r.top()),
                r.right_top(),
                egui::pos2(r.right(), c.y),
                r.right_bottom(),
                egui::pos2(c.x, r.bottom()),
                r.left_bottom(),
                egui::pos2(r.left(), c.y),
            ];
            for hp in pts {
                let hr = egui::Rect::from_center_size(hp, egui::vec2(7.0, 7.0));
                painter.rect_filled(hr, 1.0, egui::Color32::WHITE);
                painter.rect_stroke(
                    hr,
                    1.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 158, 255)),
                    egui::StrokeKind::Inside,
                );
            }
            self.size_tag(
                &painter,
                r,
                (sel.width().round() as u32).max(1),
                (sel.height().round() as u32).max(1),
            );
        } else if self.drag != Drag::Select {
            painter.text(
                egui::pos2(screen.center().x, screen.top() + 42.0),
                egui::Align2::CENTER_TOP,
                tr(strings::DRAG_A_REGION_CLICK_A_WINDOW_RIGHT_CLICK_OR_ESC),
                egui::FontId::proportional(15.0),
                egui::Color32::from_white_alpha(210),
            );
        }

        if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
            let guide = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(70));
            if self.drag == Drag::Select || self.sel.is_none() {
                painter.line_segment(
                    [
                        egui::pos2(screen.left(), p.y),
                        egui::pos2(screen.right(), p.y),
                    ],
                    guide,
                );
                painter.line_segment(
                    [
                        egui::pos2(p.x, screen.top()),
                        egui::pos2(p.x, screen.bottom()),
                    ],
                    guide,
                );
            }
        }

        let sel_pts = self
            .sel
            .map(|s| egui::Rect::from_min_max(to_p(s.min), to_p(s.max)));
        let mut over = self.show_toolbar(&ctx, screen, sel_pts);
        over.extend(self.show_text_editor(&ctx, rx, ry));

        let (pressed, released) =
            ctx.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_released()));
        let ptr = ctx.input(|i| i.pointer.interact_pos());
        let over_ui = ptr.is_some_and(|p| over.iter().any(|r| r.contains(p)));

        if pressed
            && !over_ui
            && let Some(p) = ptr
        {
            self.on_press(to_phys(p));
        }

        if self.drag != Drag::None {
            if let Some(p) = ptr {
                self.on_drag(to_phys(p));
            }
            if released {
                self.on_release();
            }
        }

        if let Some(p) = ptr {
            let icon = if over_ui {
                egui::CursorIcon::Default
            } else if let Some(sel) = self.sel {
                let p = to_phys(p);
                match hit_handle(sel, p, 11.0 * rx.max(ry)) {
                    Some(0) | Some(4) => egui::CursorIcon::ResizeNwSe,
                    Some(2) | Some(6) => egui::CursorIcon::ResizeNeSw,
                    Some(1) | Some(5) => egui::CursorIcon::ResizeVertical,
                    Some(_) => egui::CursorIcon::ResizeHorizontal,
                    None => match self.tool {
                        Tool::Select if sel.contains(p) => egui::CursorIcon::Move,
                        Tool::Select => egui::CursorIcon::Crosshair,
                        Tool::Text => egui::CursorIcon::Text,
                        _ => egui::CursorIcon::Crosshair,
                    },
                }
            } else {
                egui::CursorIcon::Crosshair
            };
            ctx.set_cursor_icon(icon);
        }

        self.keys(&ctx);

        if ctx.input(|i| i.pointer.secondary_clicked()) && !over_ui {
            if self.text_idx.is_some() {
                self.commit_text();
            } else {
                self.cancelled = true;
            }
        }
    }

    pub(crate) fn size_tag(&self, painter: &egui::Painter, r: egui::Rect, w: u32, h: u32) {
        let text = format!("{w} × {h}");
        let mut pos = egui::pos2(r.left(), r.top() - 26.0);
        if pos.y < 4.0 {
            pos.y = r.top() + 6.0;
        }
        let galley =
            painter.layout_no_wrap(text, egui::FontId::proportional(12.5), crate::theme::text());
        let bg = egui::Rect::from_min_size(pos, galley.size() + egui::vec2(10.0, 4.0));
        painter.rect_filled(bg, 4.0, crate::theme::card());
        painter.rect_stroke(
            bg,
            4.0,
            egui::Stroke::new(1.0, crate::theme::line()),
            egui::StrokeKind::Outside,
        );
        painter.galley(pos + egui::vec2(5.0, 2.0), galley, crate::theme::text());
    }
}
