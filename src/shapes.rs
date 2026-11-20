//! Annotation model, egui preview painting and raster export.

use crate::capture::{Frame, PhysRect};

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
        /// (x, y, rgb) squares covering the brushed area.
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

/// Arrow geometry shared by the preview and the exporter.
fn arrow_parts(a: Px, b: Px, w: f32) -> (Px, Px, [Px; 3]) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt().max(0.001);
    let (ux, uy) = (dx / len, dy / len);
    let head_len = (w * 4.0).clamp(14.0, len * 0.6);
    let head_w = (w * 3.0).clamp(10.0, head_len * 0.85);
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

// ---------------------------------------------------------------------------
// Preview (egui)
// ---------------------------------------------------------------------------

/// Paint shapes that are stored in frame pixels onto an egui painter whose
/// points-per-pixel scale is `ratio` (physical pixels / points).
pub fn paint(painter: &egui::Painter, shape: &Shape, ratio: f32, origin: egui::Pos2) {
    let tp = |p: Px| egui::pos2(origin.x + p.0 / ratio, origin.y + p.1 / ratio);
    let s = |w: f32| (w / ratio).max(0.6);
    match shape {
        Shape::Rect { a, b, color, w } => {
            let r = egui::Rect::from_two_pos(tp(*a), tp(*b));
            painter.rect_stroke(
                r,
                0.0,
                egui::Stroke::new(s(*w), *color),
                egui::StrokeKind::Inside,
            );
        }
        Shape::Arrow { a, b, color, w } => {
            let (line_end, _, tri) = arrow_parts(*a, *b, *w);
            painter.line_segment([tp(*a), tp(line_end)], egui::Stroke::new(s(*w), *color));
            painter.add(egui::Shape::convex_polygon(
                tri.iter().map(|p| tp(*p)).collect(),
                *color,
                egui::Stroke::NONE,
            ));
        }
        Shape::Pen { pts, color, w } => {
            if pts.len() == 1 {
                painter.circle_filled(tp(pts[0]), s(*w) * 0.5, *color);
            } else {
                let path: Vec<egui::Pos2> = pts.iter().map(|p| tp(*p)).collect();
                painter.add(egui::Shape::line(path, egui::Stroke::new(s(*w), *color)));
            }
        }
        Shape::Mosaic { cell, blocks } => {
            let step = s(*cell);
            for (x, y, rgb) in blocks {
                let r = egui::Rect::from_min_size(tp((*x, *y)), egui::vec2(step + 0.4, step + 0.4));
                painter.rect_filled(r, 0.0, egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            }
        }
        Shape::Text {
            pos,
            text,
            color,
            size,
        } => {
            painter.text(
                tp(*pos),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::proportional(size / ratio),
                *color,
            );
        }
        Shape::Number {
            center,
            n,
            color,
            r,
        } => {
            painter.circle_filled(tp(*center), s(*r), *color);
            painter.text(
                tp(*center),
                egui::Align2::CENTER_CENTER,
                n.to_string(),
                egui::FontId::proportional(1.35 * s(*r)),
                egui::Color32::WHITE,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Export (tiny-skia + fontdue)
// ---------------------------------------------------------------------------

fn sk_color(c: Col) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
}

/// Crop `sel` out of `frame` and burn all annotations into it.
pub fn render(frame: &Frame, sel: PhysRect, shapes: &[Shape]) -> image::RgbaImage {
    let w = sel.w.max(1) as u32;
    let h = sel.h.max(1) as u32;
    let fw = frame.w as i32;

    // --- base = cropped screenshot (straight alpha, opaque) ---
    let mut base = vec![0u8; (w as usize) * (h as usize) * 4];
    for y in 0..h as i32 {
        let sy = sel.y + y;
        let srow = ((sy * fw + sel.x) * 4) as usize;
        let drow = (y as usize) * (w as usize) * 4;
        let n = (w as usize) * 4;
        if srow + n <= frame.rgba.len() {
            base[drow..drow + n].copy_from_slice(&frame.rgba[srow..srow + n]);
        }
    }

    let mut layer = tiny_skia::Pixmap::new(w, h).expect("pixmap");

    // Draw accumulated vector layer into base, then clear it.
    let flush = |layer: &mut tiny_skia::Pixmap, base: &mut Vec<u8>| {
        let src = layer.data();
        for (dst, s) in base
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(src.as_chunks::<4>().0)
        {
            let a = s[3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            for c in 0..3 {
                dst[c] = (s[c] as f32 + dst[c] as f32 * (1.0 - a)).round().min(255.0) as u8;
            }
        }
        layer.fill(tiny_skia::Color::TRANSPARENT);
    };

    let offset = (sel.x as f32, sel.y as f32);

    for shape in shapes {
        match shape {
            Shape::Rect { a, b, color, w: sw } => {
                let mut pb = tiny_skia::PathBuilder::new();
                let (x0, x1) = (a.0.min(b.0) - offset.0, a.0.max(b.0) - offset.0);
                let (y0, y1) = (a.1.min(b.1) - offset.1, a.1.max(b.1) - offset.1);
                pb.move_to(x0, y0);
                pb.line_to(x1, y0);
                pb.line_to(x1, y1);
                pb.line_to(x0, y1);
                pb.close();
                if let Some(path) = pb.finish() {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color(sk_color(*color));
                    paint.anti_alias = true;
                    let stroke = tiny_skia::Stroke {
                        width: *sw,
                        line_join: tiny_skia::LineJoin::Miter,
                        ..Default::default()
                    };
                    layer.stroke_path(
                        &path,
                        &paint,
                        &stroke,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
            Shape::Arrow { a, b, color, w: sw } => {
                let (line_end, _, tri) = arrow_parts(*a, *b, *sw);
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(sk_color(*color));
                paint.anti_alias = true;

                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(a.0 - offset.0, a.1 - offset.1);
                pb.line_to(line_end.0 - offset.0, line_end.1 - offset.1);
                if let Some(path) = pb.finish() {
                    let stroke = tiny_skia::Stroke {
                        width: *sw,
                        line_cap: tiny_skia::LineCap::Round,
                        line_join: tiny_skia::LineJoin::Round,
                        ..Default::default()
                    };
                    layer.stroke_path(
                        &path,
                        &paint,
                        &stroke,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }

                let mut pb = tiny_skia::PathBuilder::new();
                pb.move_to(tri[0].0 - offset.0, tri[0].1 - offset.1);
                pb.line_to(tri[1].0 - offset.0, tri[1].1 - offset.1);
                pb.line_to(tri[2].0 - offset.0, tri[2].1 - offset.1);
                pb.close();
                if let Some(path) = pb.finish() {
                    layer.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
            }
            Shape::Pen { pts, color, w } => {
                if pts.is_empty() {
                    continue;
                }
                let mut paint = tiny_skia::Paint::default();
                paint.set_color(sk_color(*color));
                paint.anti_alias = true;
                let stroke = tiny_skia::Stroke {
                    width: *w,
                    line_cap: tiny_skia::LineCap::Round,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                };
                if pts.len() == 1 {
                    let mut pb = tiny_skia::PathBuilder::new();
                    let r = *w * 0.5;
                    pb.push_circle(pts[0].0 - offset.0, pts[0].1 - offset.1, r);
                    if let Some(path) = pb.finish() {
                        layer.fill_path(
                            &path,
                            &paint,
                            tiny_skia::FillRule::Winding,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                } else {
                    let mut pb = tiny_skia::PathBuilder::new();
                    pb.move_to(pts[0].0 - offset.0, pts[0].1 - offset.1);
                    for p in &pts[1..] {
                        pb.line_to(p.0 - offset.0, p.1 - offset.1);
                    }
                    if let Some(path) = pb.finish() {
                        layer.stroke_path(
                            &path,
                            &paint,
                            &stroke,
                            tiny_skia::Transform::identity(),
                            None,
                        );
                    }
                }
            }
            Shape::Mosaic { cell, blocks } => {
                flush(&mut layer, &mut base);
                for (x, y, rgb) in blocks {
                    fill_blocks(&mut base, w, h, *x - offset.0, *y - offset.1, *cell, *rgb);
                }
            }
            Shape::Text {
                pos,
                text,
                color,
                size,
            } => {
                flush(&mut layer, &mut base);
                draw_text(
                    &mut base,
                    w,
                    h,
                    (pos.0 - offset.0, pos.1 - offset.1),
                    text,
                    *color,
                    *size,
                    (w as f32 - (pos.0 - offset.0)).max(8.0),
                );
            }
            Shape::Number {
                center,
                n,
                color,
                r,
            } => {
                let mut pb = tiny_skia::PathBuilder::new();
                pb.push_circle(center.0 - offset.0, center.1 - offset.1, *r);
                if let Some(path) = pb.finish() {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color(sk_color(*color));
                    paint.anti_alias = true;
                    layer.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                }
                flush(&mut layer, &mut base);
                let label = n.to_string();
                let size = r * 1.35;
                let (tw, th) = text_size(&label, size);
                draw_text(
                    &mut base,
                    w,
                    h,
                    (
                        center.0 - offset.0 - tw * 0.5,
                        center.1 - offset.1 - th * 0.5,
                    ),
                    &label,
                    egui::Color32::WHITE,
                    size,
                    size * 3.0,
                );
            }
        }
    }
    flush(&mut layer, &mut base);

    image::RgbaImage::from_raw(w, h, base).expect("rgba buffer")
}

/// Number badge: the digit is drawn centred, so we shift by half the text box.
fn fill_blocks(base: &mut [u8], w: u32, h: u32, x: f32, y: f32, cell: f32, rgb: [u8; 3]) {
    let x0 = x.round().max(0.0) as i32;
    let y0 = y.round().max(0.0) as i32;
    let x1 = ((x + cell).round() as i32).min(w as i32);
    let y1 = ((y + cell).round() as i32).min(h as i32);
    for yy in y0..y1 {
        for xx in x0..x1 {
            let i = ((yy as u32 * w + xx as u32) * 4) as usize;
            base[i] = rgb[0];
            base[i + 1] = rgb[1];
            base[i + 2] = rgb[2];
            base[i + 3] = 255;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_text(
    base: &mut [u8],
    w: u32,
    h: u32,
    pos: (f32, f32),
    text: &str,
    color: Col,
    size: f32,
    max_width: f32,
) {
    let Some(font) = crate::fonts::raster() else {
        return;
    };
    if text.is_empty() {
        return;
    }
    let mut layout = fontdue::layout::Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown);
    layout.reset(&fontdue::layout::LayoutSettings {
        max_width: Some(max_width),
        ..Default::default()
    });
    layout.append(&[font], &fontdue::layout::TextStyle::new(text, size, 0));

    for g in layout.glyphs() {
        let (m, bitmap) = font.rasterize_config(g.key);
        let gx = pos.0 + g.x;
        let gy = pos.1 + g.y;
        for row in 0..m.height {
            for col in 0..m.width {
                let cov = bitmap[row * m.width + col] as f32 / 255.0;
                if cov <= 0.0 {
                    continue;
                }
                let px = gx as i32 + col as i32;
                let py = gy as i32 + row as i32;
                if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                    continue;
                }
                let i = ((py as u32 * w + px as u32) * 4) as usize;
                for c in 0..3 {
                    let src = match c {
                        0 => color.r(),
                        1 => color.g(),
                        _ => color.b(),
                    } as f32;
                    base[i + c] = (src * cov + base[i + c] as f32 * (1.0 - cov))
                        .round()
                        .min(255.0) as u8;
                }
            }
        }
    }
}

/// Centred text helper used for number badges in the preview export path.
pub fn text_size(text: &str, size: f32) -> (f32, f32) {
    let Some(font) = crate::fonts::raster() else {
        return (0.0, 0.0);
    };
    let mut width: f32 = 0.0;
    for ch in text.chars() {
        width += font.metrics(ch, size).advance_width;
    }
    (width, font.metrics('0', size).height as f32)
}
