use std::collections::HashSet;
use tokio::sync::RwLock;

/// 既定トピック集合をカプセル化し、ロック操作を一元化するレジストリ。
pub struct DefaultTopicsRegistry {
    topics: RwLock<HashSet<String>>,
}

impl DefaultTopicsRegistry {
    pub fn with_topics<I>(topics: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut set = HashSet::new();
        for topic in topics {
            let topic = topic.trim().to_string();
            if !topic.is_empty() {
                set.insert(topic);
            }
        }
        Self {
            topics: RwLock::new(set),
        }
    }

    pub async fn replace_with_single(&self, topic: String) {
        let mut guard = self.topics.write().await;
        guard.clear();
        if !topic.trim().is_empty() {
            guard.insert(topic);
        }
    }

    pub async fn replace_all<I>(&self, topics: I)
    where
        I: IntoIterator<Item = String>,
    {
        let mut guard = self.topics.write().await;
        guard.clear();
        for topic in topics {
            let topic = topic.trim().to_string();
            if !topic.is_empty() {
                guard.insert(topic);
            }
        }
    }

    pub async fn add(&self, topic: String) {
        let normalized = topic.trim();
        if normalized.is_empty() {
            return;
        }
        let mut guard = self.topics.write().await;
        guard.insert(normalized.to_string());
    }

    pub async fn remove(&self, topic: &str) {
        let mut guard = self.topics.write().await;
        guard.remove(topic);
    }

    pub async fn list(&self) -> Vec<String> {
        let guard = self.topics.read().await;
        guard.iter().cloned().collect()
    }

    pub async fn snapshot(&self) -> HashSet<String> {
        let guard = self.topics.read().await;
        guard.clone()
    }
}
