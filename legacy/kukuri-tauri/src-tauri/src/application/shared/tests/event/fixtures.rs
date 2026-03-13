use nostr_sdk::prelude::*;

pub fn sample_signed_event() -> Event {
    let keys = Keys::generate();
    EventBuilder::text_note("Test content")
        .tags(vec![Tag::hashtag("test")])
        .sign_with_keys(&keys)
        .expect("sign text note")
}
