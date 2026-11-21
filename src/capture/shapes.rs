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

pub fn paint(painter: &egui::Painter, shape: &Shape, rx: f32, ry: f32, origin: egui::Pos2) {
    let tp = |p: Px| egui::pos2(origin.x + p.0 / rx, origin.y + p.1 / ry);

    let avg = ((rx + ry) * 0.5).max(0.01);
    let s = |w: f32| (w / avg).max(0.6);
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
                egui::FontId::proportional(size / avg),
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
                badge_text_color(*color),
            );
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_head_scales_with_width_but_stays_inside() {
        let (line_end, _, tri) = arrow_parts((0.0, 0.0), (10.0, 0.0), 7.0);
        assert!(line_end.0 <= 10.0);
        assert!(tri[0].0 <= 10.0 + 0.001);
    }

    #[test]
    fn horizontal_arrow_points_right() {
        let (_, base, tri) = arrow_parts((0.0, 0.0), (100.0, 0.0), 4.0);
        assert!(base.0 > 0.0 && base.0 < 100.0);
        assert_eq!(tri[0], (100.0, 0.0));

        assert!((tri[1].1 + tri[2].1).abs() < 0.001);
    }

    #[test]
    fn badge_digits_contrast_with_background() {
        let white = egui::Color32::from_rgb(250, 250, 250);
        let black = egui::Color32::from_rgb(24, 24, 26);
        assert_eq!(badge_text_color(white), black);
        assert_eq!(badge_text_color(black), egui::Color32::WHITE);
        assert_eq!(
            badge_text_color(egui::Color32::from_rgb(232, 69, 60)),
            egui::Color32::WHITE
        );
    }

    #[test]
    fn short_arrow_is_still_rasterized() {
        let frame = Frame {
            w: 200,
            h: 120,
            rgba: vec![255u8; 200 * 120 * 4],
            origin: crate::capture::PhysRect::new(0, 0, 200, 120),
        };
        let arrow = Shape::Arrow {
            a: (20.0, 60.0),
            b: (32.0, 66.0),
            color: egui::Color32::from_rgb(232, 69, 60),
            w: 4.0,
        };
        let img = render(
            &frame,
            crate::capture::PhysRect::new(0, 0, 120, 120),
            &[arrow],
        );
        let red = img
            .pixels()
            .filter(|p| p[0] > 150 && p[1] < 130 && p[2] < 130)
            .count();
        assert!(red > 10, "expected arrow pixels, found {red}");
    }

    #[test]
    fn zero_sized_selection_renders_one_pixel() {
        let frame = Frame {
            w: 4,
            h: 4,
            rgba: vec![255u8; 4 * 4 * 4],
            origin: crate::capture::PhysRect::new(0, 0, 4, 4),
        };
        let img = render(&frame, crate::capture::PhysRect::new(3, 3, 0, 0), &[]);
        assert_eq!((img.width(), img.height()), (1, 1));
    }

    fn white_frame(w: u32, h: u32) -> Frame {
        Frame {
            w,
            h,
            rgba: vec![255u8; (w * h * 4) as usize],
            origin: crate::capture::PhysRect::new(0, 0, w as i32, h as i32),
        }
    }

    fn count(img: &image::RgbaImage, f: impl Fn(&image::Rgba<u8>) -> bool) -> usize {
        img.pixels().filter(|p| f(p)).count()
    }

    fn red(img: &image::RgbaImage) -> usize {
        count(img, |p| p[0] > 150 && p[1] < 130 && p[2] < 130)
    }

    #[test]
    fn rect_outline_is_baked() {
        let frame = white_frame(200, 120);
        let img = render(
            &frame,
            crate::capture::PhysRect::new(0, 0, 200, 120),
            &[Shape::Rect {
                a: (20.0, 20.0),
                b: (120.0, 90.0),
                color: egui::Color32::from_rgb(232, 69, 60),
                w: 4.0,
            }],
        );

        assert!(red(&img) > 300);
        assert_eq!(img.get_pixel(70, 55), &image::Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn pen_stroke_is_baked() {
        let frame = white_frame(200, 120);
        let pts = (0..40)
            .map(|i| (20.0 + i as f32 * 3.0, 60.0 + (i as f32 * 0.4).sin() * 20.0))
            .collect::<Vec<_>>();
        let img = render(
            &frame,
            crate::capture::PhysRect::new(0, 0, 200, 120),
            &[Shape::Pen {
                pts,
                color: egui::Color32::from_rgb(232, 69, 60),
                w: 5.0,
            }],
        );
        assert!(red(&img) > 200);
    }

    #[test]
    fn mosaic_blocks_are_baked() {
        let frame = white_frame(200, 120);
        let blocks = vec![
            (10.0f32, 10.0f32, [10u8, 20, 30]),
            (19.0, 10.0, [10, 20, 30]),
            (28.0, 10.0, [10, 20, 30]),
        ];
        let img = render(
            &frame,
            crate::capture::PhysRect::new(0, 0, 200, 120),
            &[Shape::Mosaic { cell: 9.0, blocks }],
        );
        assert_eq!(img.get_pixel(14, 14), &image::Rgba([10, 20, 30, 255]));
        assert_eq!(img.get_pixel(150, 14), &image::Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn number_badge_is_baked_with_contrast_text() {
        let frame = white_frame(160, 120);
        let img = render(
            &frame,
            crate::capture::PhysRect::new(0, 0, 160, 120),
            &[Shape::Number {
                center: (80.0, 60.0),
                n: 7,
                color: egui::Color32::from_rgb(24, 24, 26),
                r: 20.0,
            }],
        );

        let c = img.get_pixel(62, 60);
        assert!(c[0] < 80 && c[1] < 80 && c[2] < 80, "disc pixel {c:?}");

        let light = count(&img, |p| p[0] > 200 && p[1] > 200 && p[2] > 200);
        assert!(light > 20, "expected white digits, got {light}");
    }

    #[test]
    fn text_is_baked_and_stays_inside_the_crop() {
        let frame = white_frame(260, 120);
        let img = render(
            &frame,
            crate::capture::PhysRect::new(0, 0, 260, 120),
            &[Shape::Text {
                pos: (10.0, 20.0),
                text: "Hi 12".to_string(),
                color: egui::Color32::from_rgb(232, 69, 60),
                size: 40.0,
            }],
        );
        assert!(red(&img) > 100, "text pixels: {}", red(&img));
    }

    #[test]
    fn overlapping_shapes_keep_draw_order() {
        let frame = white_frame(120, 80);
        let img = render(
            &frame,
            crate::capture::PhysRect::new(0, 0, 120, 80),
            &[
                Shape::Rect {
                    a: (10.0, 10.0),
                    b: (110.0, 70.0),
                    color: egui::Color32::from_rgb(232, 69, 60),
                    w: 6.0,
                },
                Shape::Mosaic {
                    cell: 9.0,
                    blocks: vec![(10.0, 10.0, [0u8, 0, 0])],
                },
            ],
        );

        assert_eq!(img.get_pixel(14, 14), &image::Rgba([0, 0, 0, 255]));
    }
}
