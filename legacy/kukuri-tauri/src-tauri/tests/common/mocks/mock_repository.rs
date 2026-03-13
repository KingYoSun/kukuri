use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Note: These types would need to be imported from the actual application
// For now, we'll use placeholder types
type Post = serde_json::Value;
type Topic = serde_json::Value;
type User = serde_json::Value;
type Event = serde_json::Value;

pub struct MockPostRepository {
    posts: Arc<RwLock<HashMap<String, Post>>>,
}

impl MockPostRepository {
    pub fn new() -> Self {
        Self {
            posts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_posts(posts: Vec<Post>) -> Self {
        let mut map = HashMap::new();
        for post in posts {
            if let Some(id) = post.get("id").and_then(|v| v.as_str()) {
                map.insert(id.to_string(), post);
            }
        }
        Self {
            posts: Arc::new(RwLock::new(map)),
        }
    }
}

pub struct MockTopicRepository {
    topics: Arc<RwLock<HashMap<String, Topic>>>,
}

impl MockTopicRepository {
    pub fn new() -> Self {
        Self {
            topics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_topics(topics: Vec<Topic>) -> Self {
        let mut map = HashMap::new();
        for topic in topics {
            if let Some(id) = topic.get("id").and_then(|v| v.as_str()) {
                map.insert(id.to_string(), topic);
            }
        }
        Self {
            topics: Arc::new(RwLock::new(map)),
        }
    }
}

pub struct MockUserRepository {
    users: Arc<RwLock<HashMap<String, User>>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_users(users: Vec<User>) -> Self {
        let mut map = HashMap::new();
        for user in users {
            if let Some(npub) = user.get("npub").and_then(|v| v.as_str()) {
                map.insert(npub.to_string(), user);
            }
        }
        Self {
            users: Arc::new(RwLock::new(map)),
        }
    }
}

pub struct MockEventRepository {
    events: Arc<RwLock<HashMap<String, Event>>>,
}

impl MockEventRepository {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_events(events: Vec<Event>) -> Self {
        let mut map = HashMap::new();
        for event in events {
            if let Some(id) = event.get("id").and_then(|v| v.as_str()) {
                map.insert(id.to_string(), event);
            }
        }
        Self {
            events: Arc::new(RwLock::new(map)),
        }
    }
}