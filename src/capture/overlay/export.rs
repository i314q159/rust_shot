use super::*;

impl Session {
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
        self.outcome = Some(Outcome { action, image: img });
    }
}
