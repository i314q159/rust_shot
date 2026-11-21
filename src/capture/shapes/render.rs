use super::*;
use crate::capture::{Frame, PhysRect};

pub fn render(frame: &Frame, sel: PhysRect, shapes: &[Shape]) -> image::RgbaImage {
    let w = sel.w.max(1) as u32;
    let h = sel.h.max(1) as u32;
    let fw = frame.w as i32;

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
                    badge_text_color(*color),
                    size,
                    size * 3.0,
                );
            }
        }
    }
    flush(&mut layer, &mut base);

    image::RgbaImage::from_raw(w, h, base).expect("rgba buffer")
}

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
    let Some(font) = crate::config::fonts::raster() else {
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

pub fn text_size(text: &str, size: f32) -> (f32, f32) {
    let Some(font) = crate::config::fonts::raster() else {
        return (0.0, 0.0);
    };
    let mut width: f32 = 0.0;
    for ch in text.chars() {
        width += font.metrics(ch, size).advance_width;
    }
    (width, font.metrics('0', size).height as f32)
}
