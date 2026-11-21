use super::*;

impl App {
    pub(crate) fn refresh_theme(&mut self, ctx: &egui::Context) {
        if self.theme_pref != crate::theme::Preference::System {
            return;
        }
        let dark = crate::theme::system_is_dark();
        if dark == crate::theme::is_dark() {
            return;
        }
        crate::theme::apply(ctx, dark);
        crate::platform::util::log(&format!("theme -> {}", if dark { "dark" } else { "light" }));
        ctx.request_repaint();
    }

    pub(crate) fn set_theme(&mut self, ctx: &egui::Context, pref: crate::theme::Preference) {
        self.theme_pref = pref;
        let dark = crate::theme::resolve(pref);
        crate::theme::apply(ctx, dark);
        self.persist();
        crate::platform::util::log(&format!(
            "theme pref -> {} ({})",
            pref.key(),
            if dark { "dark" } else { "light" }
        ));
        self.status = tr1(strings::THEME, pref.label());
        ctx.request_repaint();
    }

    pub(crate) fn set_lang(&mut self, ctx: &egui::Context, pref: crate::config::i18n::Preference) {
        self.lang_pref = pref;
        crate::config::i18n::set(crate::config::i18n::resolve(pref));
        self.persist();

        if let Some(m) = &self.menu {
            m.region.set_text(tr(strings::REGION_CAPTURE));
            m.full.set_text(tr(strings::FULL_SCREEN));
            m.panel.set_text(tr(strings::SHOW_PANEL));
            m.quit.set_text(tr(strings::QUIT));
        }
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(tr(strings::RUSTSHOT)));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(panel_title().to_owned()));
        crate::platform::util::log(&format!(
            "lang pref -> {} ({})",
            pref.key(),
            crate::config::i18n::lang().key()
        ));
        self.status = tr1(strings::LANGUAGE, crate::config::i18n::lang().label());
        ctx.request_repaint();
    }

    pub(crate) fn manual_proxy(&self) -> Option<crate::config::proxy::Proxy> {
        let host = self.proxy_host.trim();
        let port = self
            .proxy_port
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)?;
        if host.is_empty() {
            return None;
        }
        Some(crate::config::proxy::Proxy::Manual {
            host: host.to_owned(),
            port,
        })
    }

    pub(crate) fn apply_proxy(&mut self, mode: u8) {
        self.proxy_mode = mode;
        match mode {
            1 => {
                self.proxy = crate::config::proxy::Proxy::Direct;
                self.proxy_bad = false;
            }
            2 => {
                if let Some(parsed) = self.manual_proxy() {
                    self.proxy = parsed;
                    self.proxy_bad = false;
                } else {
                    self.proxy_bad = true;
                    self.status =
                        tr(strings::INVALID_PROXY_ADDRESS_KEEPING_THE_CURRENT_ONE).to_owned();
                    return;
                }
            }
            _ => {
                self.proxy = crate::config::proxy::Proxy::System;
                self.proxy_bad = false;
            }
        }
        match &self.proxy {
            crate::config::proxy::Proxy::Manual { host, port } => {
                self.proxy_host = host.clone();
                self.proxy_port = port.to_string();
            }
            _ => {
                self.proxy_host.clear();
                self.proxy_port.clear();
            }
        }
        self.persist();
        crate::platform::util::log(&format!("proxy -> {}", self.proxy.key()));
        self.status = tr1(strings::PROXY, self.proxy.label());
    }

    pub(crate) fn start_install(&mut self) {
        if self.install_rx.is_some() {
            return;
        }
        let UpdateStatus::Newer(rel) = self.update.clone() else {
            return;
        };
        self.install = Install::Downloading { got: 0, total: 0 };
        self.status = tr(strings::DOWNLOADING_THE_UPDATE).to_owned();
        let (tx, rx) = std::sync::mpsc::channel();
        self.install_rx = Some(rx);
        crate::platform::util::log(&format!("update: downloading v{}", rel.version));

        let _ = std::thread::Builder::new()
            .name("rustshot-update-dl".into())
            .spawn(move || {
                let asset = crate::update::setup_name(&rel.version);
                let dest = crate::update::update_dir().join(&asset);
                let result = (|| -> Result<(), String> {
                    let expected = crate::update::expected_sha(&rel.version, &asset)?;
                    let url = crate::update::asset_url(&rel.version, &asset);
                    let _ = tx.send(InstallEvent::Progress(0, 0));

                    let mut attempt = 0;
                    while let Err(e) = crate::update::download(&url, &dest, |got, total| {
                        let _ = tx.send(InstallEvent::Progress(got, total));
                        crate::app::request_repaint();
                    }) {
                        attempt += 1;
                        if attempt >= 2 {
                            return Err(e);
                        }
                        crate::platform::util::log(&format!(
                            "update: download failed, retrying: {e}"
                        ));
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        let _ = tx.send(InstallEvent::Progress(0, 0));
                    }
                    let _ = tx.send(InstallEvent::Verifying);
                    crate::app::request_repaint();
                    let actual = crate::update::sha256_file(&dest)?;
                    if actual != expected {
                        return Err(
                            tr(strings::THE_DOWNLOAD_DOES_NOT_MATCH_SHA256SUMS_TXT).to_owned()
                        );
                    }
                    crate::update::apply::spawn_helper(&dest)
                })();
                match result {
                    Ok(()) => {
                        let _ = tx.send(InstallEvent::Done);
                    }
                    Err(e) => {
                        crate::platform::util::log(&format!("update: install aborted: {e}"));
                        let _ = tx.send(InstallEvent::Failed(e));
                    }
                }
                crate::app::request_repaint();
            });
    }
}
