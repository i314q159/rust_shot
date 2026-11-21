use std::ffi::c_void;
use std::io::Write;
use std::path::Path;

use crate::config::i18n::{strings, tr, tr1, tr2};
use windows::Win32::Networking::WinHttp::{
    WINHTTP_FLAG_SECURE, WINHTTP_QUERY_CONTENT_LENGTH, WinHttpConnect, WinHttpOpenRequest,
    WinHttpQueryDataAvailable, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
    WinHttpSetTimeouts,
};
use windows::core::PCWSTR;

use super::*;

pub(crate) use http::http_get;
use http::{Handle, header_u64, net_err, split_https, status_code, utf16};

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

mod http;
