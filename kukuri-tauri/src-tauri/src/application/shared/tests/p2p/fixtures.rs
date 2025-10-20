use crate::domain::entities::Event;

pub fn nostr_to_domain(ev: &nostr_sdk::Event) -> Event {
    let created_at =
        chrono::DateTime::<chrono::Utc>::from_timestamp(ev.created_at.as_u64() as i64, 0)
            .expect("invalid timestamp in nostr event");
    Event {
        id: ev.id.to_string(),
        pubkey: ev.pubkey.to_string(),
        created_at,
        kind: ev.kind.as_u16() as u32,
        tags: ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
        content: ev.content.clone(),
        sig: ev.sig.to_string(),
    }
}
