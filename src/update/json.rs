use crate::config::i18n::{strings, tr, tr1};

use super::*;

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
