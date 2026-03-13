use serde::{Deserialize, Serialize};

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

impl EventKind {
    pub fn from_u32(value: u32) -> Option<Self> {
        Some(value.into())
    }

    pub fn as_u32(self) -> u32 {
        self.into()
    }
}
