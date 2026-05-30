use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::TopicId;

const PUBLIC_TOPIC_RENDEZVOUS_DOMAIN: &str = "kukuri:rendezvous:public-topic:v1";
const PRIVATE_TOPIC_RENDEZVOUS_DOMAIN: &[u8] = b"kukuri:rendezvous:private-topic:v1";

pub fn public_topic_rendezvous_key(topic: &TopicId) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(PUBLIC_TOPIC_RENDEZVOUS_DOMAIN.as_bytes());
    hasher.update(b"\0");
    hasher.update(topic.as_str().as_bytes());
    hex::encode(hasher.finalize().as_bytes())
}

pub fn private_topic_rendezvous_key_hex_secret(
    namespace_secret_hex: &str,
    topic: &TopicId,
) -> Result<String> {
    let namespace_secret =
        hex::decode(namespace_secret_hex).context("invalid private rendezvous namespace secret")?;
    private_topic_rendezvous_key(namespace_secret.as_slice(), topic)
}

pub fn private_topic_rendezvous_key(namespace_secret: &[u8], topic: &TopicId) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(namespace_secret)
        .context("invalid private rendezvous namespace secret")?;
    mac.update(PRIVATE_TOPIC_RENDEZVOUS_DOMAIN);
    mac.update(b"\0");
    mac.update(topic.as_str().as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendezvous_keys_are_opaque_and_domain_separated() {
        let topic = TopicId::new("kukuri:topic:raw");
        let public = public_topic_rendezvous_key(&topic);
        let private = private_topic_rendezvous_key_hex_secret(
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
            &topic,
        )
        .expect("private key");

        assert_eq!(public.len(), 64);
        assert_eq!(private.len(), 64);
        assert_ne!(public, private);
        assert!(!public.contains(topic.as_str()));
        assert!(!private.contains(topic.as_str()));
    }
}
