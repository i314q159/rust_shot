use std::path::Path;

use crate::config::i18n::{strings, tr1};

use super::net::http_get;
use super::*;

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
