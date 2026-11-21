//! Network proxy for the update check / download.
//!
//! `proxy=` in `config.toml` (and `帮助 → 网络代理`) decides how WinHTTP talks to
//! GitHub: follow Windows, go direct, or use an explicit `host:port`.  The
//! session is opened here so both the JSON check and the installer download
//! share exactly one policy.

use crate::config::i18n::strings;
use std::ffi::c_void;

use windows::Win32::Networking::WinHttp::{
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_NAMED_PROXY,
    WINHTTP_ACCESS_TYPE_NO_PROXY, WinHttpCloseHandle, WinHttpOpen,
};
use windows::core::PCWSTR;

use crate::config::i18n::{tr, tr1, tr3};

/// Network proxy used by the update checker and the downloader.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Proxy {
    /// Whatever Windows is configured with (the default).
    #[default]
    System,
    /// No proxy at all.
    Direct,
    /// An explicit `host:port`.
    Manual { host: String, port: u16 },
}

impl Proxy {
    /// Parse the `proxy=` value written in `config.toml`.
    ///
    /// Empty → system; `direct` → no proxy; `7890` → `127.0.0.1:7890`;
    /// `host:port` → manual. Anything else is ignored (system).
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        if text.is_empty() {
            return Self::System;
        }
        let lower = text.to_ascii_lowercase();
        if lower == "direct" || lower == "none" || lower == "off" {
            return Self::Direct;
        }
        if let Ok(port) = text.parse::<u16>()
            && port > 0
        {
            return Self::Manual {
                host: "127.0.0.1".to_owned(),
                port,
            };
        }
        if let Some((host, port)) = text.rsplit_once(':') {
            let host = host.trim();
            if !host.is_empty()
                && let Ok(port) = port.trim().parse::<u16>()
                && port > 0
            {
                return Self::Manual {
                    host: host.to_owned(),
                    port,
                };
            }
        }
        Self::System
    }

    /// `host:port` for a manual proxy, otherwise `None`.
    pub fn address(&self) -> Option<String> {
        match self {
            Self::Manual { host, port } => Some(format!("{host}:{port}")),
            _ => None,
        }
    }

    /// One-line description for the dialog / error messages.
    pub fn label(&self) -> String {
        match self {
            Self::System => tr(strings::SYSTEM_PROXY).to_owned(),
            Self::Direct => tr(strings::DIRECT_NO_PROXY).to_owned(),
            Self::Manual { host, port } => format!("{host}:{port}"),
        }
    }

    /// Value written back to `config.toml`.
    pub fn key(&self) -> String {
        match self {
            Self::System => "system".to_owned(),
            Self::Direct => "direct".to_owned(),
            Self::Manual { host, port } => format!("{host}:{port}"),
        }
    }

    /// The setting currently in `config.toml` (read on demand, so a hand edit
    /// applies to the next check without restarting).
    pub fn current() -> Self {
        crate::config::Config::load().proxy
    }
}

/// Owned WinHTTP session; closed on drop.
pub struct Session(pub *mut c_void);

impl Drop for Session {
    fn drop(&mut self) {
        if !self.0.is_null() {
            let _ = unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

/// Open a WinHTTP session with `agent` as the user agent, applying [`Proxy`].
pub fn open(agent: &str) -> Result<Session, String> {
    open_with(&Proxy::current(), agent)
}

/// [`open`] with an explicit proxy (used by tests and the dialog preview).
pub fn open_with(proxy: &Proxy, agent: &str) -> Result<Session, String> {
    // WinHttpOpen rejects NAMED_PROXY unless the proxy string is passed here,
    // so the address goes straight into the call.
    let access = match proxy {
        Proxy::System => WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
        Proxy::Direct => WINHTTP_ACCESS_TYPE_NO_PROXY,
        Proxy::Manual { .. } => WINHTTP_ACCESS_TYPE_NAMED_PROXY,
    };
    let agent_w = crate::platform::util::wide(agent);
    let proxy_w = proxy.address().map(|a| crate::platform::util::wide(&a));
    let proxy_ptr = proxy_w
        .as_ref()
        .map(|w| PCWSTR(w.as_ptr()))
        .unwrap_or_else(PCWSTR::null);
    let raw = unsafe {
        WinHttpOpen(
            PCWSTR(agent_w.as_ptr()),
            access,
            proxy_ptr,
            PCWSTR::null(),
            0,
        )
    };
    crate::platform::util::log(&format!("net: proxy={}", proxy.key()));
    if raw.is_null() {
        return Err(tr3(
            strings::NET_ERROR,
            tr(strings::CANNOT_OPEN_A_NETWORK_SESSION),
            unsafe { windows::Win32::Foundation::GetLastError().0 },
            proxy.key(),
        ));
    }
    Ok(Session(raw))
}

/// Text appended to network errors so the dialog shows which proxy was used.
pub fn hint() -> String {
    tr1(strings::PROXY_2, Proxy::current().label())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_parse() {
        assert_eq!(Proxy::parse(""), Proxy::System);
        assert_eq!(Proxy::parse("   "), Proxy::System);
        assert_eq!(Proxy::parse("direct"), Proxy::Direct);
        assert_eq!(Proxy::parse("DIRECT"), Proxy::Direct);
        assert_eq!(
            Proxy::parse("7890"),
            Proxy::Manual {
                host: "127.0.0.1".into(),
                port: 7890
            }
        );
        assert_eq!(
            Proxy::parse("127.0.0.1:12334"),
            Proxy::Manual {
                host: "127.0.0.1".into(),
                port: 12334
            }
        );
        assert_eq!(
            Proxy::parse("proxy.local:8080"),
            Proxy::Manual {
                host: "proxy.local".into(),
                port: 8080
            }
        );
        // Port 0 / garbage / missing host keep the safe default.
        assert_eq!(Proxy::parse("0"), Proxy::System);
        assert_eq!(Proxy::parse("127.0.0.1:0"), Proxy::System);
        assert_eq!(Proxy::parse("127.0.0.1:70000"), Proxy::System);
        assert_eq!(Proxy::parse(":8080"), Proxy::System);
        assert_eq!(Proxy::parse("nonsense"), Proxy::System);
    }

    #[test]
    fn session_opens_with_every_mode() {
        for proxy in [
            Proxy::System,
            Proxy::Direct,
            Proxy::Manual {
                host: "127.0.0.1".into(),
                port: 12334,
            },
        ] {
            match open_with(&proxy, "RustShot/test") {
                Ok(_) => {}
                Err(e) => panic!("{proxy:?}: {e}"),
            }
        }
    }

    #[test]
    fn address_and_key_round_trip() {
        let manual = Proxy::Manual {
            host: "127.0.0.1".into(),
            port: 12334,
        };
        assert_eq!(manual.address().as_deref(), Some("127.0.0.1:12334"));
        assert_eq!(manual.key(), "127.0.0.1:12334");
        assert_eq!(Proxy::parse(&manual.key()), manual);
        assert_eq!(Proxy::System.address(), None);
        assert_eq!(Proxy::Direct.address(), None);
        assert_eq!(Proxy::System.key(), "system");
    }
}
