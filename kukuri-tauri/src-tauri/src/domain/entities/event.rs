use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::domain::value_objects::EventId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[repr(u32)]
pub enum EventKind {
    Metadata = 0,
    TextNote = 1,
    RecommendRelay = 2,
    Contacts = 3,
    EncryptedDirectMessage = 4,
    EventDeletion = 5,
    Repost = 6,
    Reaction = 7,
    BadgeAward = 8,
    ChannelCreation = 40,
    ChannelMetadata = 41,
    ChannelMessage = 42,
    ChannelHideMessage = 43,
    ChannelMuteUser = 44,
    Custom(u32),
}

impl From<u32> for EventKind {
    fn from(value: u32) -> Self {
        match value {
            0 => EventKind::Metadata,
            1 => EventKind::TextNote,
            2 => EventKind::RecommendRelay,
            3 => EventKind::Contacts,
            4 => EventKind::EncryptedDirectMessage,
            5 => EventKind::EventDeletion,
            6 => EventKind::Repost,
            7 => EventKind::Reaction,
            8 => EventKind::BadgeAward,
            40 => EventKind::ChannelCreation,
            41 => EventKind::ChannelMetadata,
            42 => EventKind::ChannelMessage,
            43 => EventKind::ChannelHideMessage,
            44 => EventKind::ChannelMuteUser,
            v => EventKind::Custom(v),
        }
    }
}

impl From<EventKind> for u32 {
    fn from(value: EventKind) -> Self {
        match value {
            EventKind::Metadata => 0,
            EventKind::TextNote => 1,
            EventKind::RecommendRelay => 2,
            EventKind::Contacts => 3,
            EventKind::EncryptedDirectMessage => 4,
            EventKind::EventDeletion => 5,
            EventKind::Repost => 6,
            EventKind::Reaction => 7,
            EventKind::BadgeAward => 8,
            EventKind::ChannelCreation => 40,
            EventKind::ChannelMetadata => 41,
            EventKind::ChannelMessage => 42,
            EventKind::ChannelHideMessage => 43,
            EventKind::ChannelMuteUser => 44,
            EventKind::Custom(v) => v,
        }
    }
}

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
            created_at: chrono::Utc::now(),
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


impl EventKind {
    pub fn as_u32(&self) -> u32 {
        u32::from(*self)
    }

    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(EventKind::Metadata),
            1 => Some(EventKind::TextNote),
            2 => Some(EventKind::RecommendRelay),
            3 => Some(EventKind::Contacts),
            4 => Some(EventKind::EncryptedDirectMessage),
            5 => Some(EventKind::EventDeletion),
            6 => Some(EventKind::Repost),
            7 => Some(EventKind::Reaction),
            8 => Some(EventKind::BadgeAward),
            40 => Some(EventKind::ChannelCreation),
            41 => Some(EventKind::ChannelMetadata),
            42 => Some(EventKind::ChannelMessage),
            43 => Some(EventKind::ChannelHideMessage),
            44 => Some(EventKind::ChannelMuteUser),
            _ => None,
        }
    }
}