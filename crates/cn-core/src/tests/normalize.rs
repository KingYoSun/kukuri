#[test]
fn http_url_normalization_trims_trailing_slash() {
    let normalized = crate::normalize_http_url("https://example.com/").expect("normalize");
    assert_eq!(normalized, "https://example.com");
}

#[test]
fn ws_url_normalization_trims_trailing_slash() {
    let normalized = crate::normalize_ws_url("wss://example.com/relay/").expect("normalize");
    assert_eq!(normalized, "wss://example.com/relay");
}
