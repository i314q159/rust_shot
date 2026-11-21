//! Update check against the public GitHub releases API.
//!
//! Deliberately dependency free: WinHTTP (shipped with Windows) performs the
//! HTTPS request and a small scanner pulls the handful of string fields we need
//! out of the release JSON, so the binary does not grow a TLS stack or serde.

pub(crate) mod apply;
pub(crate) mod sha256;

use crate::config::i18n::strings;
use std::ffi::c_void;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use crate::config::i18n::{tr, tr1, tr2};
use windows::Win32::Networking::WinHttp::{
    WINHTTP_FLAG_SECURE, WINHTTP_OPTION_URL, WINHTTP_QUERY_CONTENT_LENGTH,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpQueryHeaders, WinHttpQueryOption,
    WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
};
use windows::core::PCWSTR;

/// GitHub repository shown in the UI.
pub const REPO: &str = "i314q159/rust_shot";
/// Release page opened from the update dialog (also used as a manual fallback).
pub const RELEASES_URL: &str = "https://github.com/i314q159/rust_shot/releases/latest";
/// API endpoint queried by the check.
const API_URL: &str = "https://api.github.com/repos/i314q159/rust_shot/releases/latest";
/// Version of the running build; every check compares against it.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// State of the update check, driving both the panel badge and the dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not checked yet in this session.
    Idle,
    /// A background request is in flight.
    Checking,
    /// The running build is the newest published release.
    Latest(String),
    /// A newer release exists.
    Newer(Release),
    Failed(String),
}

/// What a found update is allowed to do on its own (`auto_update=`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoUpdate {
    /// Only report it; the user decides in the dialog.
    #[default]
    Notice,
    /// Download, verify and install without asking (still one UAC prompt).
    Silent,
}

impl AutoUpdate {
    pub fn key(self) -> &'static str {
        match self {
            Self::Notice => "notice",
            Self::Silent => "silent",
        }
    }

    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "silent" | "auto" | "yes" => Self::Silent,
            _ => Self::Notice,
        }
    }
}

/// The `latest` release as reported by GitHub.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Release {
    /// Normalised dotted version, e.g. `0.2.0`.
    pub version: String,
    pub tag: String,
    pub name: String,
    pub url: String,
    /// `YYYY-MM-DD` of the (UTC) publication date.
    pub published: String,
    pub notes: String,
}

/// Ask GitHub for the latest release on a background thread.
///
/// The caller polls the returned receiver; the thread wakes the UI itself.
pub fn check_async() -> Receiver<Status> {
    let (tx, rx) = channel();
    let _ = std::thread::Builder::new()
        .name("rustshot-update".into())
        .spawn(move || {
            let status = evaluate(fetch_latest());
            crate::platform::util::log(&format!("update check: {status:?}"));
            let _ = tx.send(status);
            crate::app::request_repaint();
        });
    rx
}

fn evaluate(result: Result<Release, String>) -> Status {
    match result {
        Ok(rel) if is_newer(&rel.version, CURRENT) => Status::Newer(rel),
        Ok(rel) => Status::Latest(rel.version),
        Err(e) => Status::Failed(e),
    }
}

/// Where GitHub publishes the files of a release tag.
pub fn asset_url(version: &str, name: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/v{version}/{name}")
}

/// File names published with every release (see `.github/workflows/release.yml`).
pub fn setup_name(version: &str) -> String {
    format!("RustShot-Setup-{version}.exe")
}
pub const SUMS_NAME: &str = "SHA256SUMS.txt";

/// `%TEMP%\RustShot\update` — downloads land here and are swapped in later.
pub fn update_dir() -> PathBuf {
    std::env::temp_dir().join("RustShot").join("update")
}

/// Is this exe part of an installed copy (next to `unins000.exe`)?
///
/// Only installed copies self-update; a portable exe is just a bare file, so
/// the update dialog points those users at the releases page instead.
pub fn installed_copy() -> bool {
    std::env::current_exe()
        .map(|exe| installed_copy_at(&exe))
        .unwrap_or(false)
}

/// [`installed_copy`] for an arbitrary exe path (the updater helper runs from
/// `%TEMP%`, so it has to ask about the app, not about itself).
pub fn installed_copy_at(exe: &Path) -> bool {
    exe.parent()
        .map(|dir| dir.join("unins000.exe").exists())
        .unwrap_or(false)
}

/// `4.5 MB` style size for the download progress line.
pub fn human_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Pick the hash for `name` out of a `SHA256SUMS.txt` payload.
///
/// Lines look like `<hex>  <file name>` (two spaces, as produced by the
/// release workflow). Unknown names and malformed lines are skipped.
pub fn checksum_for(sums: &str, name: &str) -> Option<String> {
    for line in sums.lines() {
        let line = line.trim();
        let Some((hash, file)) = line.split_once("  ") else {
            continue;
        };
        if file.trim() != name {
            continue;
        }
        let hash = hash.trim().to_ascii_lowercase();
        if hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(hash);
        }
    }
    None
}

/// SHA-256 of a file, lowercase hex.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| tr1(strings::CANNOT_READ_THE_DOWNLOADED_FILE, e))?;
    let mut hasher = crate::update::sha256::Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        use std::io::Read;
        let n = file
            .read(&mut buf)
            .map_err(|e| tr1(strings::CANNOT_READ_THE_DOWNLOADED_FILE, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish_hex())
}

/// Expected hash of one release asset, read from `SHA256SUMS.txt`.
pub fn expected_sha(version: &str, asset: &str) -> Result<String, String> {
    let response = http_get(&asset_url(version, SUMS_NAME))?;
    if response.status != 200 {
        return Err(tr1(
            strings::COULD_NOT_FETCH_THE_CHECKSUM_LIST_HTTP,
            response.status,
        ));
    }
    checksum_for(&response.body, asset)
        .ok_or_else(|| tr1(strings::THE_CHECKSUM_LIST_DOES_NOT_MENTION, asset))
}

/// Download `url` to `dest`, reporting `(received, total)` as it goes.
///
/// `total` is 0 when the server omits Content-Length. A failed or partial
/// download is deleted so a retry never picks up a half file.
pub fn download(url: &str, dest: &Path, mut progress: impl FnMut(u64, u64)) -> Result<(), String> {
    let (host, path) = split_https(url)?;
    let host_w = crate::platform::util::wide(&host);
    let path_w = crate::platform::util::wide(&path);
    let verb = crate::platform::util::wide("GET");
    let headers = utf16(&format!(
        "Accept: application/octet-stream\r\nUser-Agent: RustShot/{CURRENT}\r\n"
    ));

    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| tr1(strings::CANNOT_CREATE_THE_DOWNLOAD_FOLDER, e))?;
    }
    let mut file =
        std::fs::File::create(dest).map_err(|e| tr1(strings::CANNOT_WRITE_THE_DOWNLOAD_2, e))?;

    let result = (|| -> Result<(), String> {
        let session = crate::config::proxy::open(&format!("RustShot/{CURRENT}"))?;
        // Downloads are big compared to the JSON check: give them more time.
        let _ = unsafe { WinHttpSetTimeouts(session.0, 10000, 15000, 15000, 60000) };
        let conn = Handle::new(
            unsafe { WinHttpConnect(session.0, PCWSTR(host_w.as_ptr()), 443, 0) },
            &format!(
                "{}{}",
                tr(strings::CANNOT_REACH_GITHUB),
                crate::config::proxy::hint()
            ),
        )?;
        let request = Handle::new(
            unsafe {
                WinHttpOpenRequest(
                    conn.0,
                    PCWSTR(verb.as_ptr()),
                    PCWSTR(path_w.as_ptr()),
                    PCWSTR::null(),
                    PCWSTR::null(),
                    std::ptr::null(),
                    WINHTTP_FLAG_SECURE,
                )
            },
            tr(strings::CANNOT_CREATE_THE_DOWNLOAD_REQUEST),
        )?;
        unsafe { WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0) }
            .map_err(net_err(tr(strings::SENDING_THE_DOWNLOAD_REQUEST_FAILED)))?;
        unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) }
            .map_err(net_err(tr(strings::READING_THE_DOWNLOAD_RESPONSE_FAILED)))?;

        let status = status_code(request.0);
        if status != 200 {
            return Err(tr1(strings::THE_UPDATE_DOWNLOAD_FAILED_HTTP, status));
        }
        let total = header_u64(request.0, WINHTTP_QUERY_CONTENT_LENGTH);

        let mut received = 0u64;
        let mut last_report = 0u64;
        loop {
            let mut available = 0u32;
            unsafe { WinHttpQueryDataAvailable(request.0, &mut available) }
                .map_err(net_err(tr(strings::READING_THE_DOWNLOAD_RESPONSE_FAILED)))?;
            if available == 0 {
                break;
            }
            let chunk = available.min(64 * 1024) as usize;
            let mut buf = vec![0u8; chunk];
            let mut read = 0u32;
            unsafe {
                WinHttpReadData(
                    request.0,
                    buf.as_mut_ptr() as *mut c_void,
                    chunk as u32,
                    &mut read,
                )
            }
            .map_err(net_err(tr(strings::READING_THE_DOWNLOAD_RESPONSE_FAILED)))?;
            if read == 0 {
                break;
            }
            file.write_all(&buf[..read as usize])
                .map_err(|e| tr1(strings::CANNOT_WRITE_THE_DOWNLOAD, e))?;
            received += read as u64;
            // Report at most ~30 times a second, plus the final byte.
            if received - last_report >= 128 * 1024 {
                last_report = received;
                progress(received, total);
            }
        }
        file.flush()
            .map_err(|e| tr1(strings::CANNOT_WRITE_THE_DOWNLOAD, e))?;
        progress(received, total);
        if received == 0 {
            return Err(tr(strings::THE_DOWNLOADED_FILE_IS_EMPTY).to_owned());
        }
        Ok(())
    })();

    if result.is_err() {
        drop(file);
        let _ = std::fs::remove_file(dest);
    }
    result
}

/// Numeric response header (`WINHTTP_QUERY_* | WINHTTP_QUERY_FLAG_NUMBER`).
fn header_u64(request: *mut c_void, query: u32) -> u64 {
    let mut value = 0u32;
    let mut len = std::mem::size_of::<u32>() as u32;
    let mut index = 0u32;
    let ok = unsafe {
        WinHttpQueryHeaders(
            request,
            query | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut value as *mut u32 as *mut c_void),
            &mut len,
            &mut index,
        )
    };
    if ok.is_err() { 0 } else { value as u64 }
}

/// Open a URL with the user's default browser.
pub fn open_page(url: &str) {
    let _ = std::process::Command::new("explorer").arg(url).spawn();
    crate::platform::util::log(&format!("opened {url}"));
}

/// Fetch and parse the latest release.
///
/// The releases API is the primary source (it carries the notes). `api.github.com`
/// is blocked or throttled on some networks, so the public releases page — which
/// redirects to the newest tag — is used as a fallback.
pub fn fetch_latest() -> Result<Release, String> {
    // Debug builds can replay a captured payload (`RUSTSHOT_UPDATE_JSON`) so the
    // "update available" dialog is verifiable without bumping the version.
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os("RUSTSHOT_UPDATE_JSON") {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| tr1(strings::COULD_NOT_READ_THE_TEST_PAYLOAD, e))?;
        return parse_release(&text);
    }
    let api = fetch_from_api();
    if let Ok(rel) = &api {
        return Ok(rel.clone());
    }
    match fetch_from_page() {
        Ok(rel) => Ok(rel),
        Err(page_err) => {
            let api_err = api.unwrap_err();
            Err(tr2(
                strings::THE_RELEASES_PAGE_FAILED_TOO,
                api_err,
                page_err,
            ))
        }
    }
}

/// `https://api.github.com/repos/…/releases/latest`.
fn fetch_from_api() -> Result<Release, String> {
    let response = http_get(API_URL)?;
    match response.status {
        200 => parse_release(&response.body),
        403 | 429 => Err(tr(strings::GITHUB_RATE_LIMITED_THE_REQUEST).to_owned()),
        404 => Err(tr(strings::NO_RELEASES_FOUND).to_owned()),
        code => Err(tr1(strings::THE_GITHUB_API_RETURNED_HTTP, code)),
    }
}

/// `https://github.com/<repo>/releases/latest`, which redirects to the newest tag.
fn fetch_from_page() -> Result<Release, String> {
    let response = http_get(RELEASES_URL)?;
    if response.status != 200 {
        return Err(tr1(
            strings::THE_RELEASES_PAGE_RETURNED_HTTP,
            response.status,
        ));
    }
    let tag = tag_from_release_url(&response.url)
        .ok_or_else(|| tr(strings::NO_VERSION_IN_THE_RELEASES_PAGE_URL).to_owned())?;
    let version =
        normalize_version(&tag).ok_or_else(|| tr1(strings::CANNOT_PARSE_VERSION, &tag))?;
    Ok(Release {
        version,
        tag,
        name: String::new(),
        url: response.url,
        published: String::new(),
        notes: String::new(),
    })
}

/// `…/releases/tag/v0.2.0?x=1` → `v0.2.0`.
pub fn tag_from_release_url(url: &str) -> Option<String> {
    let tail = url.split(['?', '#']).next()?;
    let (_, tag) = tail.split_once("/releases/tag/")?;
    let tag = tag.trim_end_matches('/');
    if tag.is_empty() || tag.contains('/') {
        None
    } else {
        Some(tag.to_owned())
    }
}

/// `v1.2.3` / `1.2.3` / `release-1.2` → `1.2.3` / `1.2`. `None` when there is no
/// leading number at all.
pub fn normalize_version(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let trimmed = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);
    let core = trimmed.split(['-', '+']).next()?;
    let mut parts = Vec::new();
    for segment in core.split('.') {
        let digits: String = segment.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        parts.push(digits.parse::<u64>().ok()?);
    }
    if parts.is_empty() {
        return None;
    }
    Some(
        parts
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("."),
    )
}

fn version_parts(text: &str) -> Vec<u64> {
    normalize_version(text)
        .map(|v| v.split('.').filter_map(|p| p.parse::<u64>().ok()).collect())
        .unwrap_or_default()
}

/// True when `latest` is a newer release than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    let a = version_parts(latest);
    let b = version_parts(current);
    if a.is_empty() {
        return false;
    }
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// Read the fields we care about from a GitHub release payload.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let fields = object_fields(json);
    let get = |key: &str| {
        fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    let tag = get("tag_name").ok_or_else(|| tr(strings::THE_PAYLOAD_HAS_NO_TAG_NAME))?;
    let version =
        normalize_version(&tag).ok_or_else(|| tr1(strings::CANNOT_PARSE_VERSION, &tag))?;
    let mut notes = get("body").unwrap_or_default().replace("\r\n", "\n");
    if notes.chars().count() > 4000 {
        notes = notes.chars().take(4000).collect::<String>() + tr(strings::TRUNCATED);
    }
    Ok(Release {
        version,
        tag,
        name: get("name").unwrap_or_default(),
        url: get("html_url")
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| RELEASES_URL.to_owned()),
        published: get("published_at")
            .map(|s| s.chars().take(10).collect())
            .unwrap_or_default(),
        notes,
    })
}

// ---------------------------------------------------------------- networking

/// RAII wrapper so every early return closes the WinHTTP handle.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void, what: &str) -> Result<Self, String> {
        if raw.is_null() {
            Err(what.to_owned())
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

fn net_err(what: &str) -> impl Fn(windows::core::Error) -> String {
    move |e| tr2(strings::UPDATE_NET_ERROR, what, e.code().0)
}

/// A finished HTTP response: status, final (post-redirect) URL and body text.
pub struct HttpResponse {
    pub status: u32,
    /// Final URL after redirects — how the releases page reveals its tag.
    pub url: String,
    pub body: String,
}

/// HTTPS GET that follows redirects and returns the body as text.
fn http_get(url: &str) -> Result<HttpResponse, String> {
    let (host, path) = split_https(url)?;
    // The `W` entry points expect NUL terminated strings (`wide`); the header
    // block is passed with an explicit length instead.
    let host_w = crate::platform::util::wide(&host);
    let path_w = crate::platform::util::wide(&path);
    let verb = crate::platform::util::wide("GET");
    // GitHub rejects requests without a User-Agent header.
    let headers = utf16(&format!(
        "Accept: application/vnd.github+json, text/html\r\n\
         User-Agent: RustShot/{CURRENT}\r\n\
         X-GitHub-Api-Version: 2022-11-28\r\n"
    ));

    let session = crate::config::proxy::open(&format!("RustShot/{CURRENT}"))?;
    let _ = unsafe { WinHttpSetTimeouts(session.0, 5000, 8000, 8000, 12000) };

    let conn = Handle::new(
        unsafe { WinHttpConnect(session.0, PCWSTR(host_w.as_ptr()), 443, 0) },
        tr(strings::CANNOT_REACH_GITHUB),
    )?;

    let request = Handle::new(
        unsafe {
            WinHttpOpenRequest(
                conn.0,
                PCWSTR(verb.as_ptr()),
                PCWSTR(path_w.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            )
        },
        tr(strings::CANNOT_CREATE_THE_UPDATE_REQUEST),
    )?;

    unsafe { WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0) }
        .map_err(net_err(tr(strings::SENDING_THE_UPDATE_REQUEST_FAILED)))?;
    unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) }
        .map_err(net_err(tr(strings::READING_THE_UPDATE_RESPONSE_FAILED)))?;

    let status = status_code(request.0);
    let url = if status == 200 {
        final_url(request.0)
    } else {
        String::new()
    };
    let body = if status == 200 {
        read_body(request.0)?
    } else {
        String::new()
    };
    Ok(HttpResponse { status, url, body })
}

fn split_https(url: &str) -> Result<(String, String), String> {
    let rest = url
        .strip_prefix("https://")
        .ok_or_else(|| tr(strings::ONLY_HTTPS_URLS_ARE_SUPPORTED).to_owned())?;
    let (host, tail) = rest.split_once('/').unwrap_or((rest, ""));
    if host.is_empty() {
        return Err(tr(strings::THE_UPDATE_URL_HAS_NO_HOST).to_owned());
    }
    let path = if tail.is_empty() {
        "/".to_owned()
    } else {
        format!("/{tail}")
    };
    Ok((host.to_owned(), path))
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// HTTP status code of a response (`0` when the query itself fails).
fn status_code(request: *mut c_void) -> u32 {
    let mut code = 0u32;
    let mut len = std::mem::size_of::<u32>() as u32;
    let mut index = 0u32;
    let ok = unsafe {
        WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some(&mut code as *mut u32 as *mut c_void),
            &mut len,
            &mut index,
        )
    };
    if ok.is_err() { 0 } else { code }
}

/// Final URL of a request, i.e. the target of the redirects it followed.
fn final_url(request: *mut c_void) -> String {
    let mut len = 0u32;
    // A null buffer asks for the required size (in bytes).
    let _ = unsafe { WinHttpQueryOption(request, WINHTTP_OPTION_URL, None, &mut len) };
    if len == 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize / 2 + 1];
    let mut got = len;
    let ok = unsafe {
        WinHttpQueryOption(
            request,
            WINHTTP_OPTION_URL,
            Some(buf.as_mut_ptr() as *mut c_void),
            &mut got,
        )
    };
    if ok.is_err() {
        return String::new();
    }
    let units = (got as usize / 2).min(buf.len());
    String::from_utf16_lossy(&buf[..units])
        .trim_end_matches('\0')
        .to_owned()
}

fn read_body(request: *mut c_void) -> Result<String, String> {
    let mut body: Vec<u8> = Vec::new();
    loop {
        let mut available = 0u32;
        unsafe { WinHttpQueryDataAvailable(request, &mut available) }
            .map_err(net_err(tr(strings::READING_THE_UPDATE_RESPONSE_FAILED)))?;
        if available == 0 {
            break;
        }
        let chunk = available.min(32 * 1024) as usize;
        let mut buf = vec![0u8; chunk];
        let mut read = 0u32;
        unsafe {
            WinHttpReadData(
                request,
                buf.as_mut_ptr() as *mut c_void,
                chunk as u32,
                &mut read,
            )
        }
        .map_err(net_err(tr(strings::READING_THE_UPDATE_RESPONSE_FAILED)))?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buf[..read as usize]);
        if body.len() > 1024 * 1024 {
            return Err(tr(strings::THE_GITHUB_RESPONSE_IS_TOO_LARGE).to_owned());
        }
    }
    String::from_utf8(body).map_err(|_| tr(strings::THE_GITHUB_RESPONSE_IS_NOT_UTF_8).to_owned())
}

// ---------------------------------------------------------------------- json

/// Top-level `"key": "value"` pairs of a JSON object. Non-string values
/// (objects, arrays, numbers…) are skipped: that is all a release payload needs.
fn object_fields(json: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = json.chars().collect();
    let mut out = Vec::new();
    let mut i = 0usize;
    skip_ws(&chars, &mut i);
    if chars.get(i) != Some(&'{') {
        return out;
    }
    i += 1;
    loop {
        skip_ws(&chars, &mut i);
        match chars.get(i) {
            Some('}') | None => break,
            Some(',') => {
                i += 1;
                continue;
            }
            _ => {}
        }
        let Some(key) = parse_string(&chars, &mut i) else {
            break;
        };
        skip_ws(&chars, &mut i);
        if chars.get(i) != Some(&':') {
            break;
        }
        i += 1;
        skip_ws(&chars, &mut i);
        if chars.get(i) == Some(&'"') {
            match parse_string(&chars, &mut i) {
                Some(value) => out.push((key, value)),
                None => break,
            }
        } else {
            skip_value(&chars, &mut i);
        }
    }
    out
}

fn skip_ws(chars: &[char], i: &mut usize) {
    while matches!(chars.get(*i), Some(' ' | '\t' | '\r' | '\n')) {
        *i += 1;
    }
}

fn parse_string(chars: &[char], i: &mut usize) -> Option<String> {
    if chars.get(*i) != Some(&'"') {
        return None;
    }
    *i += 1;
    let mut out = String::new();
    while let Some(&ch) = chars.get(*i) {
        *i += 1;
        match ch {
            '"' => return Some(out),
            '\\' => {
                let esc = *chars.get(*i)?;
                *i += 1;
                match esc {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let hi = parse_hex(chars, i, 4)?;
                        if (0xD800..0xDC00).contains(&hi) {
                            // Surrogate pair: combine with the low half.
                            let mut lo = 0u32;
                            if chars.get(*i) == Some(&'\\') && chars.get(*i + 1) == Some(&'u') {
                                *i += 2;
                                lo = parse_hex(chars, i, 4)?;
                            }
                            let code: u32 = 0x1_0000
                                + ((hi - 0xD800) << 10)
                                + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        } else {
                            out.push(char::from_u32(hi).unwrap_or('\u{fffd}'));
                        }
                    }
                    other => out.push(other),
                }
            }
            other => out.push(other),
        }
    }
    None
}

fn parse_hex(chars: &[char], i: &mut usize, count: usize) -> Option<u32> {
    let mut value = 0u32;
    for _ in 0..count {
        value = value * 16 + chars.get(*i)?.to_digit(16)?;
        *i += 1;
    }
    Some(value)
}

fn skip_value(chars: &[char], i: &mut usize) {
    match chars.get(*i) {
        Some('"') => {
            let _ = parse_string(chars, i);
        }
        Some(open @ ('{' | '[')) => {
            let close = if *open == '{' { '}' } else { ']' };
            let mut depth = 0i32;
            while let Some(&ch) = chars.get(*i) {
                if ch == '"' {
                    let _ = parse_string(chars, i);
                    continue;
                }
                if ch == *open {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                    if depth <= 0 {
                        *i += 1;
                        break;
                    }
                }
                *i += 1;
            }
        }
        _ => {
            while let Some(&ch) = chars.get(*i) {
                if matches!(ch, ',' | '}' | ']') {
                    break;
                }
                *i += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r###"{
      "url": "https://api.github.com/repos/i314q159/rust_shot/releases/1",
      "html_url": "https://github.com/i314q159/rust_shot/releases/tag/v0.2.0",
      "id": 42,
      "author": { "login": "i314q159", "name": "not the release name" },
      "name": "RustShot 0.2.0",
      "tag_name": "v0.2.0",
      "published_at": "2026-11-20T09:30:00Z",
      "assets": [ { "name": "RustShot-Setup-0.2.0.exe", "size": 1234 } ],
      "draft": false,
      "prerelease": false,
      "body": "## 更新\n- 修复托盘\r\n- 新增更新检查 \"引号\" \u4e2d\u6587 😀"
    }"###;

    #[test]
    fn parses_the_fields_we_use() {
        let rel = parse_release(SAMPLE).expect("sample parses");
        assert_eq!(rel.version, "0.2.0");
        assert_eq!(rel.tag, "v0.2.0");
        assert_eq!(rel.name, "RustShot 0.2.0");
        assert_eq!(
            rel.url,
            "https://github.com/i314q159/rust_shot/releases/tag/v0.2.0"
        );
        assert_eq!(rel.published, "2026-11-20");
        assert_eq!(
            rel.notes,
            "## 更新\n- 修复托盘\n- 新增更新检查 \"引号\" 中文 😀"
        );
    }

    #[test]
    fn nested_objects_do_not_leak_into_top_level_fields() {
        // `author.name` and `assets[].name` must not be mistaken for the release name.
        let rel = parse_release(SAMPLE).unwrap();
        assert!(!rel.name.contains("not the release name"));
        assert!(!rel.name.contains(".exe"));
    }

    #[test]
    fn missing_tag_is_an_error() {
        assert!(parse_release(r#"{"name":"x"}"#).is_err());
        assert!(parse_release("not json").is_err());
    }

    #[test]
    fn missing_html_url_falls_back_to_the_release_page() {
        let rel = parse_release(r#"{"tag_name":"v9.9.9"}"#).unwrap();
        assert_eq!(rel.url, RELEASES_URL);
        assert_eq!(rel.version, "9.9.9");
        assert!(rel.notes.is_empty());
    }

    #[test]
    fn version_comparison_is_numeric_per_segment() {
        assert!(is_newer("0.2.0", "0.1.1"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.99.99"));
        assert!(is_newer("v0.1.2", "0.1.1"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(!is_newer("garbage", "0.1.1"));
    }

    #[test]
    fn version_normalisation_strips_decorations() {
        assert_eq!(normalize_version("v1.2.3").as_deref(), Some("1.2.3"));
        assert_eq!(normalize_version(" V2.0 ").as_deref(), Some("2.0"));
        assert_eq!(normalize_version("1.2.3-beta.1").as_deref(), Some("1.2.3"));
        assert_eq!(normalize_version("1.2.3+sha.abc").as_deref(), Some("1.2.3"));
        assert_eq!(normalize_version("release"), None);
        assert_eq!(normalize_version(""), None);
    }

    #[test]
    fn https_urls_split_into_host_and_path() {
        let (host, path) = split_https("https://api.github.com/repos/a/b").unwrap();
        assert_eq!(host, "api.github.com");
        assert_eq!(path, "/repos/a/b");
        let (host, path) = split_https("https://example.com").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(path, "/");
        assert!(split_https("http://example.com").is_err());
        assert!(split_https("https://").is_err());
    }

    #[test]
    fn current_version_is_the_cargo_version() {
        assert_eq!(CURRENT, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn release_page_url_reveals_its_tag() {
        assert_eq!(
            tag_from_release_url("https://github.com/i314q159/rust_shot/releases/tag/v0.2.0")
                .as_deref(),
            Some("v0.2.0")
        );
        assert_eq!(
            tag_from_release_url("https://github.com/a/b/releases/tag/v1.0.0/?utm=1&x=2")
                .as_deref(),
            Some("v1.0.0")
        );
        assert_eq!(
            tag_from_release_url("https://github.com/a/b/releases"),
            None
        );
    }

    #[test]
    fn checksum_for_picks_the_matching_asset() {
        let sums = "aaa\n\
                    0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  RustShot-Setup-0.2.0.exe\n\
                    fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210  rust_shot.exe\n";
        assert_eq!(
            checksum_for(sums, "RustShot-Setup-0.2.0.exe").as_deref(),
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        );
        assert_eq!(
            checksum_for(sums, "rust_shot.exe").as_deref(),
            Some("fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210")
        );
        // Unknown file, malformed hash, missing separator: no false matches.
        assert!(checksum_for(sums, "RustShot-0.2.0-portable.zip").is_none());
        assert!(checksum_for("nothex  rust_shot.exe", "rust_shot.exe").is_none());
        assert!(checksum_for("0123  rust_shot.exe", "rust_shot.exe").is_none());
        assert!(checksum_for("", "rust_shot.exe").is_none());
    }

    #[test]
    fn asset_urls_point_at_the_release_tag() {
        assert_eq!(
            asset_url("0.2.0", &setup_name("0.2.0")),
            "https://github.com/i314q159/rust_shot/releases/download/v0.2.0/RustShot-Setup-0.2.0.exe"
        );
        assert_eq!(
            asset_url("0.2.0", "rust_shot.exe"),
            "https://github.com/i314q159/rust_shot/releases/download/v0.2.0/rust_shot.exe"
        );
        assert!(asset_url("0.2.0", SUMS_NAME).ends_with("/v0.2.0/SHA256SUMS.txt"));
    }

    #[test]
    fn human_size_switches_units() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(4096), "4 KB");
        assert_eq!(human_size(4_500_000), "4.3 MB");
    }

    #[test]
    fn auto_update_parses() {
        assert_eq!(AutoUpdate::parse("silent"), AutoUpdate::Silent);
        assert_eq!(AutoUpdate::parse(" Silent "), AutoUpdate::Silent);
        assert_eq!(AutoUpdate::parse("notice"), AutoUpdate::Notice);
        assert_eq!(AutoUpdate::parse("nonsense"), AutoUpdate::Notice);
        assert_eq!(AutoUpdate::default(), AutoUpdate::Notice);
        assert_eq!(
            AutoUpdate::parse(AutoUpdate::Silent.key()),
            AutoUpdate::Silent
        );
    }

    /// Network smoke test, skipped by default: `cargo test -- --ignored`.
    #[test]
    #[ignore = "需要访问 GitHub"]
    fn live_release_lookup_works() {
        let api = fetch_from_api().expect("releases API reachable");
        let page = fetch_from_page().expect("releases page reachable");
        assert_eq!(api.version, page.version, "API 与发布页应指向同一个版本");
        assert!(api.version.starts_with(char::is_numeric));
        assert!(page.url.contains("/releases/tag/"));
    }
}
