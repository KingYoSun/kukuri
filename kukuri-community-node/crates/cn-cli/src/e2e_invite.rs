use anyhow::{anyhow, Result};
use clap::Args;
use cn_core::{auth, node_key, nostr};
use cn_kip_types::{KIND_INVITE_CAPABILITY, KIP_NAMESPACE, KIP_VERSION};
use serde_json::json;
use uuid::Uuid;

#[derive(Args, Clone, Debug)]
pub struct E2eInviteArgs {
    /// Topic name or topic id used for the invite capability
    #[arg(long, default_value = "e2e-community-node-invite")]
    pub topic: String,

    /// Scope for the invite capability (must be invite)
    #[arg(long, default_value = "invite")]
    pub scope: String,

    /// Expiration window in seconds from now
    #[arg(long, default_value_t = 86400)]
    pub expires_in: i64,

    /// Maximum number of uses allowed for the invite
    #[arg(long, default_value_t = 1)]
    pub max_uses: i64,

    /// Optional nonce for the invite (defaults to UUIDv4)
    #[arg(long)]
    pub nonce: Option<String>,

    /// Pretty-print the JSON output
    #[arg(long, default_value_t = false)]
    pub pretty: bool,
}

pub fn issue_invite(args: E2eInviteArgs) -> Result<()> {
    let topic_id = canonical_topic_id(&args.topic)?;
    let scope = args.scope.trim();
    if scope != "invite" {
        return Err(anyhow!("scope must be invite"));
    }

    let now = auth::unix_seconds().unwrap_or(0) as i64;
    let expires_in = args.expires_in.max(60);
    let expires_at = now.saturating_add(expires_in);
    let max_uses = args.max_uses.max(1);

    let nonce = args.nonce.unwrap_or_else(|| Uuid::new_v4().to_string());
    let d_tag = format!("invite:{nonce}");

    let content = json!({
        "schema": "kukuri-invite-v1",
        "expires": expires_at,
        "max_uses": max_uses
    })
    .to_string();

    let tags = vec![
        vec!["t".to_string(), topic_id],
        vec!["scope".to_string(), scope.to_string()],
        vec!["d".to_string(), d_tag],
        vec!["k".to_string(), KIP_NAMESPACE.to_string()],
        vec!["ver".to_string(), KIP_VERSION.to_string()],
        vec!["exp".to_string(), expires_at.to_string()],
    ];

    let node_key_path = node_key::key_path_from_env("NODE_KEY_PATH", "data/node_key.json")?;
    let node_keys = node_key::load_or_generate(&node_key_path)?;
    let event =
        nostr::build_signed_event(&node_keys, KIND_INVITE_CAPABILITY as u16, tags, content)?;

    let output = if args.pretty {
        serde_json::to_string_pretty(&event)?
    } else {
        serde_json::to_string(&event)?
    };

    println!("{output}");
    Ok(())
}

fn canonical_topic_id(input: &str) -> Result<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("topic is empty"));
    }

    let normalized = trimmed.to_lowercase();
    let base = if normalized.starts_with(crate::TOPIC_NAMESPACE) {
        normalized
    } else {
        format!("{}{}", crate::TOPIC_NAMESPACE, normalized)
    };

    if is_hashed_topic_id(&base) {
        Ok(base)
    } else {
        Ok(crate::hash_topic_id(&base))
    }
}

fn is_hashed_topic_id(topic_id: &str) -> bool {
    topic_id
        .strip_prefix(crate::TOPIC_NAMESPACE)
        .is_some_and(|tail| tail.len() == 64 && tail.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_topic_id_hashes_plain_name() {
        let name = "e2e-invite";
        let base = format!("{}{}", crate::TOPIC_NAMESPACE, name.to_lowercase());
        let expected = crate::hash_topic_id(&base);
        let actual = canonical_topic_id(name).expect("topic id");
        assert_eq!(actual, expected);
    }

    #[test]
    fn canonical_topic_id_preserves_hashed() {
        let hashed = format!("{}{}", crate::TOPIC_NAMESPACE, "a".repeat(64));
        let actual = canonical_topic_id(&hashed).expect("topic id");
        assert_eq!(actual, hashed);
    }

    #[test]
    fn canonical_topic_id_accepts_prefixed_name() {
        let name = "kukuri:TestTopic";
        let base = format!("{}{}", crate::TOPIC_NAMESPACE, "testtopic");
        let expected = crate::hash_topic_id(&base);
        let actual = canonical_topic_id(name).expect("topic id");
        assert_eq!(actual, expected);
    }
}
