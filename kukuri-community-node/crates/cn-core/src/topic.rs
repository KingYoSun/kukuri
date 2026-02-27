use anyhow::{anyhow, Result};
use blake3::Hasher;

pub const DEFAULT_PUBLIC_TOPIC_ID: &str =
    "kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0";

pub fn normalize_topic_id(topic_id: &str) -> Result<String> {
    let trimmed = topic_id.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("topic_id is empty"));
    }
    if !trimmed.starts_with("kukuri:") {
        return Err(anyhow!("topic_id must start with kukuri:"));
    }
    Ok(trimmed.to_string())
}

pub fn topic_id_to_gossip_bytes(topic_id: &str) -> Result<[u8; 32]> {
    let normalized = normalize_topic_id(topic_id)?;
    let hex_part = normalized.strip_prefix("kukuri:").unwrap_or(&normalized);
    if hex_part.len() == 64 && hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut buf = [0u8; 32];
        hex::decode_to_slice(hex_part, &mut buf)
            .map_err(|_| anyhow!("failed to decode topic hex"))?;
        return Ok(buf);
    }

    let mut hasher = Hasher::new();
    hasher.update(normalized.as_bytes());
    Ok(*hasher.finalize().as_bytes())
}
