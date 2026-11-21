use super::*;

impl Session {
    pub(crate) fn on_press(&mut self, p: egui::Pos2) {
        if self.text_idx.is_some() {
            self.commit_text();
        }
        if let Some(sel) = self.sel {
            let tol = 11.0 * self.rx.max(self.ry);
            if let Some(h) = hit_handle(sel, p, tol) {
                self.drag = Drag::Resize {
                    handle: h,
                    sel0: sel,
                };
                return;
            }
            if sel.contains(p) {
                match self.tool {
                    Tool::Select if self.repick => {
                        self.repick = false;
                        self.begin_selection(p);
                    }
                    Tool::Select => {
                        self.drag = Drag::Move {
                            start: p,
                            sel0: sel,
                        }
                    }
                    Tool::Text => {
                        self.start_text((p.x, p.y));
                        self.drag = Drag::None;
                    }
                    Tool::Number => {
                        self.push_number((p.x, p.y));
                        self.drag = Drag::None;
                    }
                    _ => self.start_shape((p.x, p.y)),
                }
                return;
            }
        }

        if self.sel.is_some() && self.tool != Tool::Select {
            return;
        }
        self.begin_selection(p);
    }

    pub(crate) fn begin_selection(&mut self, p: egui::Pos2) {
        let p = egui::pos2(
            p.x.clamp(0.0, self.frame.w as f32),
            p.y.clamp(0.0, self.frame.h as f32),
        );
        self.anchor = Some(p);
        self.sel = Some(egui::Rect::from_min_max(p, p));
        self.drag = Drag::Select;
    }

    pub(crate) fn on_drag(&mut self, p: egui::Pos2) {
        let bounds = self.bounds();
        match self.drag {
            Drag::Select => {
                if let Some(a) = self.anchor {
                    let r = egui::Rect::from_two_pos(a, p);
                    let r = egui::Rect::from_min_max(
                        egui::pos2(
                            r.min.x.clamp(0.0, bounds.width()),
                            r.min.y.clamp(0.0, bounds.height()),
                        ),
                        egui::pos2(
                            r.max.x.clamp(0.0, bounds.width()),
                            r.max.y.clamp(0.0, bounds.height()),
                        ),
                    );
                    self.sel = Some(r);
                }
            }
            Drag::Move { start, sel0 } => {
                let d = p - start;
                let moved = sel0.translate(d);
                let clamped = clamp_rect_to(moved, bounds);
                self.sel = Some(clamped);
            }
            Drag::Resize { handle, sel0 } => {
                let mut l = sel0.left();
                let mut t = sel0.top();
                let mut r = sel0.right();
                let mut b = sel0.bottom();
                match handle {
                    0 => {
                        l = p.x;
                        t = p.y;
                    }
                    1 => t = p.y,
                    2 => {
                        r = p.x;
                        t = p.y;
                    }
                    3 => r = p.x,
                    4 => {
                        r = p.x;
                        b = p.y;
                    }
                    5 => b = p.y,
                    6 => {
                        l = p.x;
                        b = p.y;
                    }
                    _ => l = p.x,
                }
                let rect = egui::Rect::from_min_max(
                    egui::pos2(
                        l.min(r).clamp(0.0, bounds.width()),
                        t.min(b).clamp(0.0, bounds.height()),
                    ),
                    egui::pos2(
                        l.max(r).clamp(0.0, bounds.width()),
                        t.max(b).clamp(0.0, bounds.height()),
                    ),
                );
                self.sel = Some(rect);
            }
            Drag::Draw => self.update_shape((p.x, p.y)),
            Drag::None => {}
        }
    }

    pub(crate) fn on_release(&mut self) {
        match self.drag {
            Drag::Select => {
                let tol = 3.0 * self.rx.max(self.ry);
                let clicked = self
                    .sel
                    .is_none_or(|s| s.width() <= tol && s.height() <= tol);
                if clicked {
                    self.sel = Some(self.hover.unwrap_or_else(|| self.bounds()));
                }
            }
            Drag::Draw => self.finish_shape(),
            _ => {}
        }
        self.drag = Drag::None;
        self.anchor = None;
    }

    pub(crate) fn keys(&mut self, ctx: &egui::Context) {
        let (esc, enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter) && !i.modifiers.shift,
            )
        });
        if self.text_idx.is_some() {
            if esc {
                self.cancel_text();
            } else if enter {
                self.commit_text();
            }
            return;
        }
        if esc {
            self.cancelled = true;
            return;
        }

        let pick = ctx.input(|i| {
            [
                i.key_pressed(egui::Key::Num1),
                i.key_pressed(egui::Key::Num2),
                i.key_pressed(egui::Key::Num3),
                i.key_pressed(egui::Key::Num4),
                i.key_pressed(egui::Key::Num5),
                i.key_pressed(egui::Key::Num6),
                i.key_pressed(egui::Key::Num7),
            ]
        });
        if let Some(idx) = pick.iter().position(|p| *p) {
            self.tool = Tool::all()[idx];
        }

        let copy = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
        if copy {
            self.export(Export::Copy);
            return;
        }
        let cmd = ctx.input(|i| i.modifiers.command);
        if cmd {
            if ctx.input(|i| i.key_pressed(egui::Key::Z)) {
                self.undo();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::S)) {
                self.export(Export::Save);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                self.sel = Some(self.bounds());
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.export(Export::Copy);
        }
    }
}
