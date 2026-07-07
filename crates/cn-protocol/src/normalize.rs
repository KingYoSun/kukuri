use std::collections::BTreeSet;
use std::net::SocketAddr;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use url::Url;

use kukuri_core::KukuriEnvelope;

pub fn normalize_http_url(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let parsed = Url::parse(trimmed).with_context(|| format!("invalid url `{trimmed}`"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => bail!("unsupported url scheme `{other}`"),
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        bail!("url must not contain query or fragment");
    }
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

pub fn normalize_ws_url(value: &str) -> Result<String> {
    let trimmed = value.trim();
    let parsed = Url::parse(trimmed).with_context(|| format!("invalid ws url `{trimmed}`"))?;
    match parsed.scheme() {
        "ws" | "wss" => {}
        other => bail!("unsupported websocket url scheme `{other}`"),
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        bail!("websocket url must not contain query or fragment");
    }
    Ok(parsed.to_string().trim_end_matches('/').to_string())
}

pub fn normalize_http_url_list(values: Vec<String>) -> Result<Vec<String>> {
    let mut deduped = BTreeSet::new();
    for value in values {
        let normalized = normalize_http_url(value.as_str())?;
        deduped.insert(normalized);
    }
    Ok(deduped.into_iter().collect())
}

pub fn parse_auth_envelope(value: &Value) -> Result<KukuriEnvelope> {
    serde_json::from_value(value.clone()).context("invalid auth envelope json")
}

pub fn verify_auth_envelope(raw: &KukuriEnvelope) -> Result<()> {
    raw.verify()
        .context("auth envelope signature verification failed")?;
    Ok(())
}

pub fn first_tag_value<'a>(envelope: &'a KukuriEnvelope, name: &str) -> Option<&'a str> {
    envelope.tags.iter().find_map(|tag| {
        if tag.first().map(String::as_str) == Some(name) {
            tag.get(1).map(String::as_str)
        } else {
            None
        }
    })
}

pub fn normalize_pubkey(value: &str) -> Result<String> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.len() != 64 || !trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        bail!("invalid pubkey");
    }
    Ok(trimmed)
}

pub fn parse_socket_addr_env(var_name: &str, default: &str) -> Result<SocketAddr> {
    let value = std::env::var(var_name).unwrap_or_else(|_| default.to_string());
    value
        .parse::<SocketAddr>()
        .with_context(|| format!("failed to parse {var_name}"))
}
