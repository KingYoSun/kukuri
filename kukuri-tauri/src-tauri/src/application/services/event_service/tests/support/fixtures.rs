use crate::domain::entities::Event;

pub fn create_test_event() -> Event {
    Event::new(1, "Test content".to_string(), "test_pubkey".to_string())
}
