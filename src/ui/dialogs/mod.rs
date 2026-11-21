use crate::app::{APP_AUTHOR, APP_VERSION, App, Install, LICENSE_TEXT};
use crate::config::i18n::strings;
use crate::config::i18n::{tr, tr1, tr2};
use crate::update::Status as UpdateStatus;

impl App {
    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        self.update_dialog(ctx);
        self.proxy_dialog(ctx);
        self.license_dialog(ctx);
    }
}

mod license;
mod proxy;
mod release;
