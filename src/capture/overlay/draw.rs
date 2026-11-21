use super::*;

impl Session {
    pub(crate) fn start_shape(&mut self, p: Px) {
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
                cell: MOSAIC_CELL,
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

    pub(crate) fn update_shape(&mut self, p: Px) {
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

    pub(crate) fn mosaic_last(&self) -> Option<Px> {
        match self.shapes.last() {
            Some(Shape::Mosaic { blocks, .. }) => blocks
                .last()
                .map(|(x, y, _)| (*x + MOSAIC_CELL * 0.5, *y + MOSAIC_CELL * 0.5)),
            _ => None,
        }
    }

    pub(crate) fn stamp_mosaic(&mut self, from: Px, to: Px) {
        let radius = 15.0f32;
        let cell = MOSAIC_CELL;
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

    pub(crate) fn finish_shape(&mut self) {
        self.mosaic_seen.clear();
        let remove = match self.shapes.last() {
            Some(Shape::Rect { a, b, .. }) | Some(Shape::Arrow { a, b, .. }) => {
                ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt() < 6.0
            }
            Some(Shape::Pen { pts, .. }) => pts.is_empty(),
            Some(Shape::Mosaic { blocks, .. }) => blocks.is_empty(),
            _ => false,
        };
        if remove {
            self.shapes.pop();
        }
    }

    pub(crate) fn push_number(&mut self, p: Px) {
        self.counter += 1;
        let r = (self.width * 5.0).clamp(12.0, 22.0);
        self.shapes.push(Shape::Number {
            center: p,
            n: self.counter,
            color: self.color,
            r,
        });
    }
}
