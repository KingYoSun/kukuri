use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Topic {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub is_joined: bool,
    pub member_count: u32,
    pub post_count: u32,
    pub is_public: bool,
    pub owner: Option<String>,
}

impl Topic {
    pub fn new(name: String, description: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description,
            created_at: now,
            updated_at: now,
            is_joined: false,
            member_count: 0,
            post_count: 0,
            is_public: true,
            owner: None,
        }
    }

    pub fn public_topic() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: "public".to_string(),
            name: "#public".to_string(),
            description: Some("公開タイムライン".to_string()),
            created_at: now,
            updated_at: now,
            is_joined: true,
            member_count: 0,
            post_count: 0,
            is_public: true,
            owner: None,
        }
    }

    pub fn join(&mut self) {
        self.is_joined = true;
        self.member_count += 1;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    pub fn leave(&mut self) {
        self.is_joined = false;
        if self.member_count > 0 {
            self.member_count -= 1;
        }
        self.updated_at = chrono::Utc::now().timestamp();
    }

    pub fn increment_post_count(&mut self) {
        self.post_count += 1;
        self.updated_at = chrono::Utc::now().timestamp();
    }

    pub fn decrement_post_count(&mut self) {
        if self.post_count > 0 {
            self.post_count -= 1;
            self.updated_at = chrono::Utc::now().timestamp();
        }
    }

    pub fn update_description(&mut self, description: String) {
        self.description = Some(description);
        self.updated_at = chrono::Utc::now().timestamp();
    }

    pub fn set_owner(&mut self, owner: String) {
        self.owner = Some(owner);
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStats {
    pub topic_id: String,
    pub member_count: u32,
    pub post_count: u32,
    pub active_members: u32,
    pub posts_today: u32,
    pub last_activity: Option<i64>,
}

impl TopicStats {
    pub fn new(topic_id: String) -> Self {
        Self {
            topic_id,
            member_count: 0,
            post_count: 0,
            active_members: 0,
            posts_today: 0,
            last_activity: None,
        }
    }
}