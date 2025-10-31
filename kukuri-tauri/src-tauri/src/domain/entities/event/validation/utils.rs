pub(super) fn is_hex_n(s: &str, n: usize) -> bool {
    s.len() == n && s.chars().all(|c| c.is_ascii_hexdigit())
}

pub(super) fn is_ws_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("ws://") || lower.starts_with("wss://")) && lower.len() > 5
}
