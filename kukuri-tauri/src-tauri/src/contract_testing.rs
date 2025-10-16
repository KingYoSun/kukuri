use crate::domain::entities::event::Event;

pub fn validate_nip10_tags(tags: Vec<Vec<String>>) -> Result<(), String> {
    let mut event = Event::new(1, String::new(), "a".repeat(64));
    event.id = "f".repeat(64);
    event.sig = "f".repeat(128);
    event.created_at = chrono::Utc::now();
    event.tags = tags;
    event.validate_nip10_19()
}
