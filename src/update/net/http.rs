use std::ffi::c_void;

use crate::config::i18n::{strings, tr, tr2};
use windows::Win32::Networking::WinHttp::{
    WINHTTP_FLAG_SECURE, WINHTTP_OPTION_URL, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
    WinHttpConnect, WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpQueryHeaders,
    WinHttpQueryOption, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};
use windows::core::PCWSTR;

use super::*;

pub(crate) fn header_u64(request: *mut c_void, query: u32) -> u64 {
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

pub(crate) struct Handle(pub(crate) *mut c_void);

impl Handle {
    pub(crate) fn new(raw: *mut c_void, what: &str) -> Result<Self, String> {
        if raw.is_null() {
            Err(what.to_owned())
        } else {
            Ok(Self(raw))
        }
    }
}

pub(crate) fn net_err(what: &str) -> impl Fn(windows::core::Error) -> String {
    move |e| tr2(strings::UPDATE_NET_ERROR, what, e.code().0)
}

pub struct HttpResponse {
    pub status: u32,

    pub url: String,
    pub body: String,
}

pub(crate) fn http_get(url: &str) -> Result<HttpResponse, String> {
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

pub(crate) fn split_https(url: &str) -> Result<(String, String), String> {
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

pub(crate) fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

pub(crate) fn status_code(request: *mut c_void) -> u32 {
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

pub(crate) fn final_url(request: *mut c_void) -> String {
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

pub(crate) fn read_body(request: *mut c_void) -> Result<String, String> {
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
