use super::*;

impl Session {
    pub(crate) fn undo(&mut self) {
        if self.text_idx.is_some() {
            self.commit_text();
        }
        self.shapes.pop();
        if self.text_idx.is_some_and(|i| i >= self.shapes.len()) {
            self.text_idx = None;
            self.text_focus_done = false;
        }
    }
}
