mod capture;
mod general;
mod net;
mod panel;
mod update;

pub use capture::*;
pub use general::*;
pub use net::*;
pub use panel::*;
pub use update::*;

#[derive(Clone, Copy, Debug)]
pub struct Text {
    pub zh: &'static str,
    pub en: &'static str,
}

pub const LANGUAGE_ZH: &str = "中文";
pub const LANGUAGE_EN: &str = "English";
