use super::*;

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
