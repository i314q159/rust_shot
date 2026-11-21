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

pub const REPO: &str = "i314q159/rust_shot";

pub const RELEASES_URL: &str = "https://github.com/i314q159/rust_shot/releases/latest";

const API_URL: &str = "https://api.github.com/repos/i314q159/rust_shot/releases/latest";

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,

    Checking,

    Latest(String),

    Newer(Release),
    Failed(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AutoUpdate {
    #[default]
    Notice,

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub tag: String,
    pub name: String,
    pub url: String,

    pub published: String,
    pub notes: String,
}

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

pub fn asset_url(version: &str, name: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/v{version}/{name}")
}

pub fn setup_name(version: &str) -> String {
    format!("RustShot-Setup-{version}.exe")
}
pub const SUMS_NAME: &str = "SHA256SUMS.txt";

pub fn update_dir() -> PathBuf {
    std::env::temp_dir().join("RustShot").join("update")
}

pub fn installed_copy_at(exe: &Path) -> bool {
    exe.parent()
        .map(|dir| dir.join("unins000.exe").exists())
        .unwrap_or(false)
}

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

pub fn open_page(url: &str) {
    let _ = std::process::Command::new("explorer").arg(url).spawn();
    crate::platform::util::log(&format!("opened {url}"));
}

pub fn fetch_latest() -> Result<Release, String> {
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

fn fetch_from_api() -> Result<Release, String> {
    let response = http_get(API_URL)?;
    match response.status {
        200 => parse_release(&response.body),
        403 | 429 => Err(tr(strings::GITHUB_RATE_LIMITED_THE_REQUEST).to_owned()),
        404 => Err(tr(strings::NO_RELEASES_FOUND).to_owned()),
        code => Err(tr1(strings::THE_GITHUB_API_RETURNED_HTTP, code)),
    }
}

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

pub struct HttpResponse {
    pub status: u32,

    pub url: String,
    pub body: String,
}

fn http_get(url: &str) -> Result<HttpResponse, String> {
    let (host, path) = split_https(url)?;

    let host_w = crate::platform::util::wide(&host);
    let path_w = crate::platform::util::wide(&path);
    let verb = crate::platform::util::wide("GET");

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

fn final_url(request: *mut c_void) -> String {
    let mut len = 0u32;

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
