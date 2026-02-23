use super::user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub content: String,
    pub author: User,
    pub topic_id: String,
    pub thread_namespace: Option<String>,
    pub thread_uuid: Option<String>,
    pub thread_root_event_id: Option<String>,
    pub thread_parent_event_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub likes: u32,
    pub boosts: u32,
    pub replies: Vec<Post>,
    pub is_synced: bool,
    pub is_boosted: bool,
    pub is_bookmarked: bool,
    pub scope: Option<String>,
    pub epoch: Option<i64>,
    pub is_encrypted: bool,
    pub local_id: Option<String>,
    pub event_id: Option<String>,
}

impl Post {
    pub fn new(content: String, author: User, topic_id: String) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let local_id = id.clone();

        Self {
            id,
            content,
            author,
            topic_id,
            thread_namespace: None,
            thread_uuid: None,
            thread_root_event_id: None,
            thread_parent_event_id: None,
            created_at: chrono::Utc::now(),
            tags: Vec::new(),
            likes: 0,
            boosts: 0,
            replies: Vec::new(),
            is_synced: false,
            is_boosted: false,
            is_bookmarked: false,
            scope: None,
            epoch: None,
            is_encrypted: false,
            local_id: Some(local_id),
            event_id: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn mark_as_synced(&mut self, event_id: String) {
        self.is_synced = true;
        self.event_id = Some(event_id);
    }

    pub fn add_reply(&mut self, reply: Post) {
        self.replies.push(reply);
    }

    pub fn increment_likes(&mut self) {
        self.likes += 1;
    }

    pub fn decrement_likes(&mut self) {
        if self.likes > 0 {
            self.likes -= 1;
        }
    }

    pub fn increment_boosts(&mut self) {
        self.boosts += 1;
        self.is_boosted = true;
    }

    pub fn toggle_bookmark(&mut self) {
        self.is_bookmarked = !self.is_bookmarked;
    }

    pub fn new_with_id(
        id: String,
        content: String,
        author: User,
        topic_id: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            content,
            author,
            topic_id,
            thread_namespace: None,
            thread_uuid: None,
            thread_root_event_id: None,
            thread_parent_event_id: None,
            created_at,
            tags: Vec::new(),
            likes: 0,
            boosts: 0,
            replies: Vec::new(),
            is_synced: false,
            is_boosted: false,
            is_bookmarked: false,
            scope: None,
            epoch: None,
            is_encrypted: false,
            local_id: None,
            event_id: None,
        }
    }

    pub fn mark_as_unsynced(&mut self) {
        self.is_synced = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDraft {
    pub id: String,
    pub content: String,
    pub topic_id: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PostDraft {
    pub fn new(content: String, topic_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            topic_id,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = chrono::Utc::now();
    }

    pub fn into_post(self, author: User) -> Post {
        Post::new(self.content, author, self.topic_id).with_tags(self.tags)
    }
}
