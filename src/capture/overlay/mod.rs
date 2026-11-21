use crate::config::i18n::strings;
use std::collections::HashSet;
use std::sync::Arc;

use crate::capture::shapes::{self, Col, Px, Shape};
use crate::capture::{Frame, PhysRect};
use crate::config::i18n::tr;

pub fn overlay_title(serial: u64) -> String {
    format!("RustShot Overlay {serial}")
}

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
            Tool::Select => tr(strings::SELECT),
            Tool::Rect => tr(strings::RECT),
            Tool::Arrow => tr(strings::ARROW),
            Tool::Pen => tr(strings::PEN),
            Tool::Text => tr(strings::TEXT),
            Tool::Mosaic => tr(strings::MOSAIC),
            Tool::Number => tr(strings::NUMBER),
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
}

pub struct Outcome {
    pub action: Export,
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

    title: String,
    tex: Option<egui::TextureHandle>,

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

    rx: f32,
    ry: f32,

    repick: bool,

    toolbar_w: f32,
    toolbar_h: f32,
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

fn is_bright(c: egui::Color32) -> bool {
    0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32 > 150.0
}

const MOSAIC_CELL: f32 = 9.0;

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
    pub fn new(
        frame: Arc<Frame>,
        monitor: PhysRect,
        title: String,
        tool: Tool,
        hover: Option<PhysRect>,
        full_frame: bool,
    ) -> Self {
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

        let full_sel = full_frame.then(|| {
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(frame.w as f32, frame.h as f32))
        });
        Self {
            frame,
            monitor,
            title,
            tex: None,
            sel: full_sel,
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
            rx: 1.0,
            ry: 1.0,
            repick: full_frame,
            toolbar_w: 620.0,
            toolbar_h: 44.0,
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
}

mod canvas;
mod draw;
mod export;
mod history;
mod input;
mod text;
mod toolbar;
