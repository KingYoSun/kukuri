use crate::domain::entities as domain;
use crate::domain::value_objects::EventId as DomainEventId;
use anyhow::Result;

pub fn nostr_to_domain_event(nostr: &nostr_sdk::Event) -> Result<domain::Event> {
    let id_hex = nostr.id.to_string();
    let id = DomainEventId::from_hex(&id_hex).map_err(|e| anyhow::anyhow!(e))?;

    let secs = nostr.created_at.as_u64() as i64;
    let created_at = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .ok_or_else(|| anyhow::anyhow!("invalid timestamp"))?;

    let kind = nostr.kind.as_u16() as u32;
    let tags: Vec<Vec<String>> = nostr.tags.iter().map(|t| t.clone().to_vec()).collect();
    let sig = nostr.sig.to_string();

    let event = domain::Event::new_with_id(
        id,
        nostr.pubkey.to_string(),
        nostr.content.clone(),
        kind,
        tags,
        created_at,
        sig,
    );
    Ok(event)
}
