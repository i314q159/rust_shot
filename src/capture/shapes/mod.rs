pub type Px = (f32, f32);
pub type Col = egui::Color32;

#[derive(Clone, Debug)]
pub enum Shape {
    Rect {
        a: Px,
        b: Px,
        color: Col,
        w: f32,
    },
    Arrow {
        a: Px,
        b: Px,
        color: Col,
        w: f32,
    },
    Pen {
        pts: Vec<Px>,
        color: Col,
        w: f32,
    },
    Mosaic {
        cell: f32,

        blocks: Vec<(f32, f32, [u8; 3])>,
    },
    Text {
        pos: Px,
        text: String,
        color: Col,
        size: f32,
    },
    Number {
        center: Px,
        n: u32,
        color: Col,
        r: f32,
    },
}

fn arrow_parts(a: Px, b: Px, w: f32) -> (Px, Px, [Px; 3]) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (ux, uy) = (dx / len, dy / len);

    let head_len = (w * 4.0).min(len * 0.6).max(1.0);
    let head_w = (w * 3.0).min(head_len * 0.9).max(1.0);
    let tip = b;
    let base = (b.0 - ux * head_len, b.1 - uy * head_len);
    let (px, py) = (-uy, ux);
    let tri = [
        tip,
        (base.0 + px * head_w * 0.5, base.1 + py * head_w * 0.5),
        (base.0 - px * head_w * 0.5, base.1 - py * head_w * 0.5),
    ];
    let line_end = (b.0 - ux * head_len * 0.72, b.1 - uy * head_len * 0.72);
    (line_end, base, tri)
}

pub fn badge_text_color(c: Col) -> Col {
    let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if lum > 150.0 {
        egui::Color32::from_rgb(24, 24, 26)
    } else {
        egui::Color32::WHITE
    }
}

fn sk_color(c: Col) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
}

pub use paint::paint;
pub use render::render;

mod paint;
mod render;
