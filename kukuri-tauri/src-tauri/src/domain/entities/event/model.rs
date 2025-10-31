use crate::domain::value_objects::EventId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: DateTime<Utc>,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl Event {
    pub fn new(kind: u32, content: String, pubkey: String) -> Self {
        Self {
            id: String::new(),
            pubkey,
            created_at: Utc::now(),
            kind,
            tags: Vec::new(),
            content,
            sig: String::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<Vec<String>>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_tag(&mut self, tag: Vec<String>) {
        self.tags.push(tag);
    }

    pub fn add_p_tag(&mut self, pubkey: String) {
        self.tags.push(vec!["p".to_string(), pubkey]);
    }

    pub fn add_e_tag(&mut self, event_id: String) {
        self.tags.push(vec!["e".to_string(), event_id]);
    }

    pub fn add_t_tag(&mut self, hashtag: String) {
        self.tags.push(vec!["t".to_string(), hashtag]);
    }

    pub fn get_referenced_event_ids(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "e")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn get_referenced_pubkeys(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "p")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn get_hashtags(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "t")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn new_with_id(
        id: EventId,
        pubkey: String,
        content: String,
        kind: u32,
        tags: Vec<Vec<String>>,
        created_at: DateTime<Utc>,
        sig: String,
    ) -> Self {
        Self {
            id: id.to_hex(),
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig,
        }
    }
}
