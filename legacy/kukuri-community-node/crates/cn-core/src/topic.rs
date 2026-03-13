use anyhow::{anyhow, Result};
use blake3::Hasher;

pub const DEFAULT_PUBLIC_TOPIC_ID: &str =
    "kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0";
const TOPIC_NAMESPACE: &str = "kukuri:tauri:";
const LEGACY_TOPIC_NAMESPACE: &str = "kukuri:";

pub fn normalize_topic_id(topic_id: &str) -> Result<String> {
    let trimmed = topic_id.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("topic_id is empty"));
    }
    let normalized = trimmed.to_lowercase();
    if normalized.starts_with(TOPIC_NAMESPACE) || normalized.starts_with(LEGACY_TOPIC_NAMESPACE) {
        return Ok(normalized);
    }
    if !normalized.starts_with("kukuri:") {
        return Err(anyhow!("topic_id must start with kukuri:"));
    }
    Ok(normalized)
}

pub fn topic_id_to_gossip_bytes(topic_id: &str) -> Result<[u8; 32]> {
    let normalized = normalize_topic_id(topic_id)?;
    let canonical_hex_part = normalized.strip_prefix(TOPIC_NAMESPACE);
    let legacy_hex_part = normalized.strip_prefix(LEGACY_TOPIC_NAMESPACE);
    if let Some(hex_part) = canonical_hex_part
        .or(legacy_hex_part)
        .filter(|hex_part| hex_part.len() == 64 && hex_part.chars().all(|c| c.is_ascii_hexdigit()))
    {
        let mut buf = [0u8; 32];
        hex::decode_to_slice(hex_part, &mut buf)
            .map_err(|_| anyhow!("failed to decode topic hex"))?;
        return Ok(buf);
    }

    let mut hasher = Hasher::new();
    hasher.update(normalized.as_bytes());
    Ok(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::{normalize_topic_id, topic_id_to_gossip_bytes, DEFAULT_PUBLIC_TOPIC_ID};

    #[test]
    fn normalize_topic_id_preserves_legacy_namespace() {
        assert_eq!(
            normalize_topic_id("kukuri:public").expect("normalize legacy topic id"),
            "kukuri:public"
        );
        assert_eq!(
            normalize_topic_id("kukuri:tauri:public").expect("normalize canonical topic id"),
            "kukuri:tauri:public"
        );
    }

    #[test]
    fn topic_id_to_gossip_bytes_decodes_hashed_kukuri_tauri_topic_ids() {
        let bytes = topic_id_to_gossip_bytes(DEFAULT_PUBLIC_TOPIC_ID).expect("topic bytes");
        let expected =
            hex::decode("731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0")
                .expect("hex decode");
        assert_eq!(bytes.as_slice(), expected.as_slice());
    }
}
