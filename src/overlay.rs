//! The full-screen frozen-image overlay: region selection + annotation.

use std::collections::HashSet;
use std::sync::Arc;

use crate::capture::{Frame, PhysRect};
use crate::shapes::{self, Col, Px, Shape};

pub const OVERLAY_TITLE: &str = "RustShot Overlay";

pub const PALETTE: [Col; 6] = [
    egui::Color32::from_rgb(232, 69, 60),
    egui::Color32::from_rgb(247, 201, 72),
    egui::Color32::from_rgb(47, 191, 113),
    egui::Color32::from_rgb(42, 127, 255),
    egui::Color32::from_rgb(250, 250, 250),
    egui::Color32::from_rgb(24, 24, 26),
];

pub const WIDTHS: [f32; 3] = [2.0, 4.0, 7.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Rect,
    Arrow,
    Pen,
    Text,
    Mosaic,
    Number,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "选择",
            Tool::Rect => "矩形",
            Tool::Arrow => "箭头",
            Tool::Pen => "画笔",
            Tool::Text => "文字",
            Tool::Mosaic => "马赛克",
            Tool::Number => "序号",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Tool::Select => "✥",
            Tool::Rect => "▭",
            Tool::Arrow => "↗",
            Tool::Pen => "✎",
            Tool::Text => "T",
            Tool::Mosaic => "▦",
            Tool::Number => "①",
        }
    }
    pub fn all() -> [Tool; 7] {
        [
            Tool::Select,
            Tool::Rect,
            Tool::Arrow,
            Tool::Pen,
            Tool::Text,
            Tool::Mosaic,
            Tool::Number,
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Export {
    Copy,
    Save,
    Pin,
}

pub struct Outcome {
    pub action: Export,
    /// Screen position (virtual desktop pixels) the image came from.
    pub origin: (i32, i32),
    pub image: image::RgbaImage,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Drag {
    None,
    Select,
    Move { start: egui::Pos2, sel0: egui::Rect },
    Resize { handle: usize, sel0: egui::Rect },
    Draw,
}

pub struct Session {
    pub frame: Arc<Frame>,
    pub monitor: PhysRect,
    tex: Option<egui::TextureHandle>,
    /// Selection in frame-local device pixels.
    sel: Option<egui::Rect>,
    anchor: Option<egui::Pos2>,
    tool: Tool,
    color: Col,
    width: f32,
    shapes: Vec<Shape>,
    drag: Drag,
    text_idx: Option<usize>,
    text_focus_done: bool,
    counter: u32,
    hover: Option<egui::Rect>,
    mosaic_seen: HashSet<(i32, i32)>,
    fix_tries: u32,
    ratio: f32,
    over_rects: Vec<egui::Rect>,
    pub outcome: Option<Outcome>,
    pub cancelled: bool,
}

fn clamp_rect_to(r: egui::Rect, bounds: egui::Rect) -> egui::Rect {
    let w = r.width().min(bounds.width());
    let h = r.height().min(bounds.height());
    let x = r.min.x.max(bounds.min.x).min(bounds.max.x - w);
    let y = r.min.y.max(bounds.min.y).min(bounds.max.y - h);
    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
}

fn hit_handle(sel: egui::Rect, p: egui::Pos2, tol: f32) -> Option<usize> {
    let c = sel.center();
    let pts = [
        sel.left_top(),
        egui::pos2(c.x, sel.top()),
        sel.right_top(),
        egui::pos2(sel.right(), c.y),
        sel.right_bottom(),
        egui::pos2(c.x, sel.bottom()),
        sel.left_bottom(),
        egui::pos2(sel.left(), c.y),
    ];
    for (i, hp) in pts.iter().enumerate() {
        if (hp.x - p.x).abs() <= tol && (hp.y - p.y).abs() <= tol {
            return Some(i);
        }
    }
    None
}

impl Session {
    pub fn new(frame: Arc<Frame>, monitor: PhysRect, tool: Tool, hover: Option<PhysRect>) -> Self {
        let hover = hover.and_then(|h| {
            let r = egui::Rect::from_min_size(
                egui::pos2((h.x - frame.origin.x) as f32, (h.y - frame.origin.y) as f32),
                egui::vec2(h.w as f32, h.h as f32),
            );
            let bounds = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(frame.w as f32, frame.h as f32),
            );
            let r = r.intersect(bounds);
            if r.width() > 8.0 && r.height() > 8.0 {
                Some(r)
            } else {
                None
            }
        });
        Self {
            frame,
            monitor,
            tex: None,
            sel: None,
            anchor: None,
            tool,
            color: PALETTE[0],
            width: WIDTHS[1],
            shapes: Vec::new(),
            drag: Drag::None,
            text_idx: None,
            text_focus_done: false,
            counter: 0,
            hover,
            mosaic_seen: HashSet::new(),
            fix_tries: 0,
            ratio: 1.0,
            over_rects: Vec::new(),
            outcome: None,
            cancelled: false,
        }
    }

    fn bounds(&self) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(self.frame.w as f32, self.frame.h as f32),
        )
    }

    // -----------------------------------------------------------------
    // painting + interaction, called once per frame inside the viewport
    // -----------------------------------------------------------------
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let screen = ctx.viewport_rect();
        if self.fix_tries == 0 {
            crate::util::log(&format!(
                "overlay frame0 screen={screen:?} frame={}x{} monitor={:?}",
                self.frame.w, self.frame.h, self.monitor
            ));
        }

        // Pin the OS window to the monitor using device pixels, no DPI math.
        if self.fix_tries < 8 {
            let ok = crate::win::force_rect_by_title(OVERLAY_TITLE, self.monitor);
            self.fix_tries += 1;
            if ok && self.fix_tries > 2 {
                crate::util::log("overlay geometry pinned");
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.request_repaint();
                self.fix_tries = 8;
                return;
            }
            ctx.request_repaint();
            return;
        }
        if screen.width() < 8.0 || screen.height() < 8.0 {
            ctx.request_repaint();
            return;
        }

        self.ratio = self.frame.w as f32 / screen.width();
        let ratio = self.ratio;
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

        let to_p = |p: egui::Pos2| egui::pos2(origin.x + p.x / ratio, origin.y + p.y / ratio);
        let to_phys =
            |p: egui::Pos2| egui::pos2((p.x - origin.x) * ratio, (p.y - origin.y) * ratio);

        // ---- dim everything outside the selection ----
        let dim = egui::Color32::from_black_alpha(96);
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
                painter.rect_filled(screen, 0.0, egui::Color32::from_black_alpha(80));
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

        // ---- annotations ----
        if let Some(sel) = self.sel {
            let clip = egui::Rect::from_min_max(to_p(sel.min), to_p(sel.max));
            let sp = painter.with_clip_rect(clip);
            for s in &self.shapes {
                shapes::paint(&sp, s, ratio, origin);
            }
        }

        // ---- selection chrome ----
        if let Some(sel) = self.sel {
            let r = egui::Rect::from_min_max(to_p(sel.min), to_p(sel.max));
            painter.rect_stroke(
                r,
                0.0,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(64, 158, 255)),
                egui::StrokeKind::Outside,
            );
            // handles
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
                "拖动鼠标框选区域 · 单击选中窗口 · 右键或 ESC 取消",
                egui::FontId::proportional(15.0),
                egui::Color32::from_white_alpha(210),
            );
        }

        // ---- crosshair guides ----
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

        // ---- toolbar ----
        let sel_pts = self
            .sel
            .map(|s| egui::Rect::from_min_max(to_p(s.min), to_p(s.max)));
        let mut over = self.show_toolbar(&ctx, screen, sel_pts);
        over.extend(self.show_text_editor(&ctx, ratio));

        // ---- input ----
        let (pressed, down, released) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
            )
        });
        let ptr = ctx.input(|i| i.pointer.interact_pos());
        let over_ui = ptr.is_some_and(|p| over.iter().any(|r| r.contains(p)));

        if pressed
            && !over_ui
            && let Some(p) = ptr
        {
            self.on_press(to_phys(p));
        }
        if down
            && !over_ui
            && self.drag != Drag::None
            && let Some(p) = ptr
        {
            self.on_drag(to_phys(p));
        }
        if released {
            self.on_release();
        }

        // cursor feedback
        if let Some(p) = ptr {
            let icon = if over_ui {
                egui::CursorIcon::Default
            } else if let Some(sel) = self.sel {
                let p = to_phys(p);
                match hit_handle(sel, p, 11.0) {
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
        self.over_rects = over;

        if ctx.input(|i| i.pointer.secondary_clicked()) && !over_ui {
            if self.text_idx.is_some() {
                self.commit_text();
            } else {
                self.cancelled = true;
            }
        }
    }

    fn size_tag(&self, painter: &egui::Painter, r: egui::Rect, w: u32, h: u32) {
        let text = format!("{w} × {h}");
        let mut pos = egui::pos2(r.left(), r.top() - 26.0);
        if pos.y < 4.0 {
            pos.y = r.top() + 6.0;
        }
        let galley =
            painter.layout_no_wrap(text, egui::FontId::proportional(12.5), egui::Color32::WHITE);
        let bg = egui::Rect::from_min_size(pos, galley.size() + egui::vec2(10.0, 4.0));
        painter.rect_filled(bg, 4.0, egui::Color32::from_black_alpha(190));
        painter.galley(pos + egui::vec2(5.0, 2.0), galley, egui::Color32::WHITE);
    }

    // -----------------------------------------------------------------
    // mouse
    // -----------------------------------------------------------------
    fn on_press(&mut self, p: egui::Pos2) {
        if self.text_idx.is_some() {
            self.commit_text();
        }
        if let Some(sel) = self.sel {
            if let Some(h) = hit_handle(sel, p, 11.0) {
                self.drag = Drag::Resize {
                    handle: h,
                    sel0: sel,
                };
                return;
            }
            if sel.contains(p) {
                match self.tool {
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
        let p = egui::pos2(
            p.x.clamp(0.0, self.frame.w as f32),
            p.y.clamp(0.0, self.frame.h as f32),
        );
        self.anchor = Some(p);
        self.sel = Some(egui::Rect::from_min_max(p, p));
        self.drag = Drag::Select;
    }

    fn on_drag(&mut self, p: egui::Pos2) {
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

    fn on_release(&mut self) {
        match self.drag {
            Drag::Select => {
                let tiny = self.sel.is_none_or(|s| s.width() < 8.0 || s.height() < 8.0);
                if tiny {
                    self.sel = Some(self.hover.unwrap_or_else(|| self.bounds()));
                }
            }
            Drag::Draw => self.finish_shape(),
            _ => {}
        }
        self.drag = Drag::None;
        self.anchor = None;
    }

    // -----------------------------------------------------------------
    // shapes
    // -----------------------------------------------------------------
    fn start_shape(&mut self, p: Px) {
        let shape = match self.tool {
            Tool::Rect => Shape::Rect {
                a: p,
                b: p,
                color: self.color,
                w: self.width,
            },
            Tool::Arrow => Shape::Arrow {
                a: p,
                b: p,
                color: self.color,
                w: self.width,
            },
            Tool::Pen => Shape::Pen {
                pts: vec![p],
                color: self.color,
                w: self.width,
            },
            Tool::Mosaic => Shape::Mosaic {
                cell: 9.0,
                blocks: Vec::new(),
            },
            _ => return,
        };
        self.shapes.push(shape);
        self.mosaic_seen.clear();
        if self.tool == Tool::Mosaic {
            self.stamp_mosaic(p, p);
        }
        self.drag = Drag::Draw;
    }

    fn update_shape(&mut self, p: Px) {
        if matches!(self.shapes.last(), Some(Shape::Mosaic { .. })) {
            let from = self.mosaic_last().unwrap_or(p);
            self.stamp_mosaic(from, p);
            return;
        }
        if let Some(last) = self.shapes.last_mut() {
            match last {
                Shape::Rect { b, .. } | Shape::Arrow { b, .. } => *b = p,
                Shape::Pen { pts, .. } => {
                    if let Some(prev) = pts.last().copied() {
                        let d = ((p.0 - prev.0).powi(2) + (p.1 - prev.1).powi(2)).sqrt();
                        if d > 1.5 {
                            pts.push(p);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn mosaic_last(&self) -> Option<Px> {
        match self.shapes.last() {
            Some(Shape::Mosaic { blocks, .. }) => {
                blocks.last().map(|(x, y, _)| (*x + 4.5, *y + 4.5))
            }
            _ => None,
        }
    }

    fn stamp_mosaic(&mut self, from: Px, to: Px) {
        let radius = 15.0f32;
        let cell = 9.0f32;
        let frame = self.frame.clone();
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        let len = (dx * dx + dy * dy).sqrt();
        let steps = ((len / (radius * 0.35)).ceil() as i32).max(1);

        let Some(Shape::Mosaic { blocks, .. }) = self.shapes.last_mut() else {
            return;
        };
        let bounds_w = frame.w as i32;
        let bounds_h = frame.h as i32;

        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let cx = from.0 + dx * t;
            let cy = from.1 + dy * t;
            let x0 = ((cx - radius) / cell).floor() as i32;
            let x1 = ((cx + radius) / cell).ceil() as i32;
            let y0 = ((cy - radius) / cell).floor() as i32;
            let y1 = ((cy + radius) / cell).ceil() as i32;
            for gy in y0..=y1 {
                for gx in x0..=x1 {
                    let px = gx as f32 * cell;
                    let py = gy as f32 * cell;
                    let ccx = px + cell * 0.5;
                    let ccy = py + cell * 0.5;
                    let d = ((ccx - cx).powi(2) + (ccy - cy).powi(2)).sqrt();
                    if d > radius * 1.05 {
                        continue;
                    }
                    if px < 0.0
                        || py < 0.0
                        || px + cell > bounds_w as f32
                        || py + cell > bounds_h as f32
                    {
                        continue;
                    }
                    if !self.mosaic_seen.insert((gx, gy)) {
                        continue;
                    }
                    // average colour of the source cell
                    let mut acc = [0u32; 3];
                    let mut n = 0u32;
                    let step = 2i32;
                    let mut sy = py as i32;
                    while sy < py as i32 + cell as i32 {
                        let mut sx = px as i32;
                        while sx < px as i32 + cell as i32 {
                            let c = frame.pixel(sx, sy);
                            acc[0] += c[0] as u32;
                            acc[1] += c[1] as u32;
                            acc[2] += c[2] as u32;
                            n += 1;
                            sx += step;
                        }
                        sy += step;
                    }
                    let n = n.max(1);
                    blocks.push((
                        px,
                        py,
                        [(acc[0] / n) as u8, (acc[1] / n) as u8, (acc[2] / n) as u8],
                    ));
                }
            }
        }
    }

    fn finish_shape(&mut self) {
        self.mosaic_seen.clear();
        let remove = match self.shapes.last() {
            Some(Shape::Rect { a, b, .. }) | Some(Shape::Arrow { a, b, .. }) => {
                ((a.0 - b.0).abs() < 4.0 && (a.1 - b.1).abs() < 4.0)
                    || ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() < 8.0
            }
            Some(Shape::Pen { pts, .. }) => pts.is_empty(),
            Some(Shape::Mosaic { blocks, .. }) => blocks.is_empty(),
            _ => false,
        };
        if remove {
            self.shapes.pop();
        }
    }

    fn push_number(&mut self, p: Px) {
        self.counter += 1;
        let r = (self.width * 5.0).clamp(12.0, 22.0);
        self.shapes.push(Shape::Number {
            center: p,
            n: self.counter,
            color: self.color,
            r,
        });
    }

    fn start_text(&mut self, p: Px) {
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

    fn commit_text(&mut self) {
        if let Some(idx) = self.text_idx.take() {
            let empty = matches!(self.shapes.get(idx), Some(Shape::Text { text, .. }) if text.trim().is_empty());
            if (empty || idx >= self.shapes.len()) && idx < self.shapes.len() {
                self.shapes.remove(idx);
            }
        }
        self.text_focus_done = false;
    }

    // -----------------------------------------------------------------
    // toolbar / text editor areas
    // -----------------------------------------------------------------
    fn show_text_editor(&mut self, ctx: &egui::Context, ratio: f32) -> Vec<egui::Rect> {
        let Some(idx) = self.text_idx else {
            return Vec::new();
        };
        let screen = ctx.viewport_rect();
        let (pos, _) = match self.shapes.get(idx) {
            Some(Shape::Text { pos, size, .. }) => (
                egui::pos2(screen.min.x + pos.0 / ratio, screen.min.y + pos.1 / ratio),
                *size,
            ),
            _ => return Vec::new(),
        };

        let mut rects = Vec::new();
        let want_focus = !self.text_focus_done;
        let shapes = &mut self.shapes;
        let area = egui::Area::new(egui::Id::unique("rustshot_text_editor"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos - egui::vec2(4.0, 4.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_black_alpha(90))
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(64, 158, 255),
                    ))
                    .corner_radius(3.0)
                    .inner_margin(egui::Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        if let Some(Shape::Text {
                            text, color, size, ..
                        }) = shapes.get_mut(idx)
                        {
                            let te = ui.add(
                                egui::TextEdit::multiline(text)
                                    .font(egui::FontId::proportional(*size / ratio))
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

    fn show_toolbar(
        &mut self,
        ctx: &egui::Context,
        screen: egui::Rect,
        sel_pts: Option<egui::Rect>,
    ) -> Vec<egui::Rect> {
        let width_guess = 620.0;
        let x = match sel_pts {
            Some(r) => (r.center().x - width_guess * 0.5)
                .clamp(8.0, (screen.right() - width_guess - 8.0).max(8.0)),
            None => screen.center().x - width_guess * 0.5,
        };
        let y = match sel_pts {
            Some(r) if r.bottom() + 62.0 < screen.bottom() => r.bottom() + 10.0,
            Some(r) if r.top() - 58.0 > screen.top() => r.top() - 54.0,
            Some(_) => screen.bottom() - 56.0,
            None => screen.bottom() - 56.0,
        };

        let mut export: Option<Export> = None;
        let mut cancel = false;
        let mut undo = false;

        let area = egui::Area::new(egui::Id::unique("rustshot_toolbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(x, y))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgb(30, 32, 36))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(64, 68, 76)))
                    .corner_radius(9.0)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 14,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(120),
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
                                    "细"
                                } else if w < 6.0 {
                                    "中"
                                } else {
                                    "粗"
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
                                        egui::Stroke::new(1.5, egui::Color32::WHITE),
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
                                        egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_white_alpha(120),
                                        ),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }
                            ui.separator();
                            if ui.button("撤销").clicked() {
                                undo = true;
                            }
                            if ui.button("复制").clicked() {
                                export = Some(Export::Copy);
                            }
                            if ui.button("保存").clicked() {
                                export = Some(Export::Save);
                            }
                            if ui.button("贴图").clicked() {
                                export = Some(Export::Pin);
                            }
                            if ui.button("取消").clicked() {
                                cancel = true;
                            }
                        });
                    });
            });

        if undo {
            self.shapes.pop();
        }
        if let Some(a) = export {
            self.export(a);
        }
        if cancel {
            self.cancelled = true;
        }

        vec![area.response.rect.expand(4.0)]
    }

    // -----------------------------------------------------------------
    // keyboard + export
    // -----------------------------------------------------------------
    fn keys(&mut self, ctx: &egui::Context) {
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if esc {
            if self.text_idx.is_some() {
                self.commit_text();
            } else {
                self.cancelled = true;
            }
            return;
        }
        if self.text_idx.is_some() {
            return;
        }
        // 1..7 pick the annotation tool.
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
        let cmd = ctx.input(|i| i.modifiers.command);
        if cmd {
            if ctx.input(|i| i.key_pressed(egui::Key::Z)) {
                self.shapes.pop();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::C)) {
                self.export(Export::Copy);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::S)) {
                self.export(Export::Save);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::T)) {
                self.export(Export::Pin);
            }
            if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                self.sel = Some(self.bounds());
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            self.export(Export::Copy);
        }
    }

    pub fn export(&mut self, action: Export) {
        if self.sel.is_none() {
            self.sel = Some(self.bounds());
        }
        let Some(sel) = self.sel else { return };
        let bounds = self.bounds();
        let sel = sel.intersect(bounds);
        if sel.width() < 1.0 || sel.height() < 1.0 {
            return;
        }
        let rect = PhysRect::new(
            sel.min.x.round() as i32,
            sel.min.y.round() as i32,
            sel.width().round().max(1.0) as i32,
            sel.height().round().max(1.0) as i32,
        );
        let img = shapes::render(&self.frame, rect, &self.shapes);
        self.outcome = Some(Outcome {
            action,
            origin: (self.frame.origin.x + rect.x, self.frame.origin.y + rect.y),
            image: img,
        });
    }
}
