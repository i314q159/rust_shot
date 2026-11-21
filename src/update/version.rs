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
