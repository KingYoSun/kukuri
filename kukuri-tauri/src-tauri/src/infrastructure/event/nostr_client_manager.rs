use anyhow::Result;
use nostr_sdk::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

fn relay_connect_wait_timeout() -> Duration {
    if cfg!(test) {
        Duration::from_millis(10)
    } else {
        Duration::from_secs(3)
    }
}

fn normalize_relay_urls(relay_urls: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for relay_url in relay_urls {
        let trimmed = relay_url.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

async fn connected_relay_urls(client: &Client) -> Vec<String> {
    client
        .relays()
        .await
        .into_iter()
        .filter_map(|(url, relay)| relay.is_connected().then(|| url.to_string()))
        .collect()
}

pub struct NostrClientManager {
    client: Arc<RwLock<Option<Client>>>,
    keys: Option<Keys>,
    configured_relays: Vec<String>,
}

impl NostrClientManager {
    pub fn new() -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            keys: None,
            configured_relays: Vec::new(),
        }
    }

    pub async fn init_with_keys(&mut self, secret_key: &SecretKey) -> Result<()> {
        let keys = Keys::new(secret_key.clone());
        self.keys = Some(keys.clone());

        let client = Client::new(keys.clone());
        self.apply_relays_to_client(&client).await?;
        *self.client.write().await = Some(client);

        info!(
            relay_count = self.configured_relays.len(),
            "Nostr client initialized with keys"
        );
        Ok(())
    }

    pub async fn replace_relays(&mut self, relay_urls: Vec<String>) -> Result<()> {
        self.configured_relays = normalize_relay_urls(relay_urls);

        let client = self.client.read().await.as_ref().cloned();
        if let Some(client) = client {
            self.apply_relays_to_client(&client).await?;
        }

        info!(
            relay_count = self.configured_relays.len(),
            "Updated Nostr relay configuration"
        );
        Ok(())
    }

    async fn apply_relays_to_client(&self, client: &Client) -> Result<()> {
        client.disconnect().await;
        client.force_remove_all_relays().await;

        for relay_url in &self.configured_relays {
            client.add_relay(relay_url).await?;
        }

        if !self.configured_relays.is_empty() {
            client.connect().await;
            client
                .wait_for_connection(relay_connect_wait_timeout())
                .await;

            let connected_relays = connected_relay_urls(client).await;
            if connected_relays.is_empty() {
                return Err(anyhow::anyhow!(
                    "No configured Nostr relay connected within {:?}: {}",
                    relay_connect_wait_timeout(),
                    self.configured_relays.join(", ")
                ));
            }
        }

        Ok(())
    }

    pub async fn relay_statuses(&self) -> Vec<(String, String)> {
        let configured_relays = self.configured_relays.clone();
        let client = self.client.read().await.as_ref().cloned();
        let mut statuses = Vec::new();
        let mut seen = HashSet::new();
        let mut client_statuses: HashMap<String, String> = HashMap::new();

        if let Some(client) = client {
            for (url, relay) in client.relays().await {
                let status = if relay.is_connected() {
                    "connected"
                } else {
                    "disconnected"
                };
                client_statuses.insert(url.to_string(), status.to_string());
            }
        }

        for relay_url in configured_relays {
            let status = client_statuses
                .get(&relay_url)
                .cloned()
                .unwrap_or_else(|| "disconnected".to_string());
            if seen.insert(relay_url.clone()) {
                statuses.push((relay_url, status));
            }
        }

        for (relay_url, status) in client_statuses {
            if seen.insert(relay_url.clone()) {
                statuses.push((relay_url, status));
            }
        }

        statuses
    }

    pub async fn disconnect(&self) -> Result<()> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            client.disconnect().await;
            info!("Disconnected from all relays");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    pub async fn publish_event(&self, event: Event) -> Result<EventId> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            if self.configured_relays.is_empty() {
                return Err(anyhow::anyhow!("no relays specified"));
            }

            if connected_relay_urls(client).await.is_empty() {
                return Err(anyhow::anyhow!("not connected to any relays"));
            }

            let output = client.send_event(&event).await?;
            let event_id = output.id();
            info!("Published event: {}", event_id);
            Ok(*event_id)
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    pub async fn subscribe(&self, filters: Vec<Filter>) -> Result<()> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            for filter in filters {
                client.subscribe(filter, None).await?;
            }
            info!("Subscribed to filters");
            Ok(())
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    pub async fn fetch_events(&self, filter: Filter, timeout: Duration) -> Result<Events> {
        let client_guard = self.client.read().await;
        if let Some(client) = client_guard.as_ref() {
            if self.configured_relays.is_empty() {
                return Err(anyhow::anyhow!("no relays specified"));
            }

            if connected_relay_urls(client).await.is_empty() {
                return Err(anyhow::anyhow!("not connected to any relays"));
            }

            let events = client.fetch_events(filter, timeout).await?;
            Ok(events)
        } else {
            Err(anyhow::anyhow!("Client not initialized"))
        }
    }

    pub fn get_public_key(&self) -> Option<PublicKey> {
        self.keys.as_ref().map(|k| k.public_key())
    }

    #[cfg(test)]
    async fn configured_relays(&self) -> Vec<String> {
        self.configured_relays.clone()
    }
}

impl Default for NostrClientManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_secret_key(ch: char) -> SecretKey {
        let hex: String = std::iter::repeat_n(ch, 64).collect();
        SecretKey::parse(&hex).expect("valid secret key")
    }

    fn sample_event(secret_key: &SecretKey) -> Event {
        let keys = Keys::new(secret_key.clone());
        EventBuilder::text_note("hello")
            .sign_with_keys(&keys)
            .expect("event")
    }

    #[tokio::test]
    async fn test_client_initialization() {
        let mut manager = NostrClientManager::new();
        let secret_key = sample_secret_key('1');

        manager
            .init_with_keys(&secret_key)
            .await
            .expect("initialize client");

        let client = manager.client.read().await;
        assert!(client.is_some());
    }

    #[tokio::test]
    async fn test_client_not_initialized_error() {
        let manager = NostrClientManager::new();
        let event = sample_event(&sample_secret_key('2'));

        let err = manager.publish_event(event).await.expect_err("should fail");
        assert!(err.to_string().contains("Client not initialized"));
    }

    #[tokio::test]
    async fn test_public_key_generation() {
        let mut manager = NostrClientManager::new();
        let secret_key = sample_secret_key('3');
        let expected = Keys::new(secret_key.clone()).public_key();

        manager
            .init_with_keys(&secret_key)
            .await
            .expect("initialize client");

        assert_eq!(manager.get_public_key(), Some(expected));
    }

    #[tokio::test]
    async fn test_client_reinitialization() {
        let mut manager = NostrClientManager::new();
        let first_secret = sample_secret_key('4');
        let second_secret = sample_secret_key('5');

        manager
            .init_with_keys(&first_secret)
            .await
            .expect("first init");
        let first_pk = manager.get_public_key().expect("first public key");

        manager
            .init_with_keys(&second_secret)
            .await
            .expect("second init");
        let second_pk = manager.get_public_key().expect("second public key");

        assert_ne!(first_pk, second_pk);
    }

    #[tokio::test]
    async fn test_replace_relays_deduplicates_before_init() {
        let mut manager = NostrClientManager::new();
        manager
            .replace_relays(vec![
                "wss://relay.example".to_string(),
                "wss://relay.example".to_string(),
            ])
            .await
            .expect("store relays");

        assert_eq!(
            manager.configured_relays().await,
            vec!["wss://relay.example".to_string()]
        );
    }

    #[tokio::test]
    async fn test_relay_statuses_report_configured_relays_when_client_is_not_initialized() {
        let mut manager = NostrClientManager::new();
        manager
            .replace_relays(vec!["ws://127.0.0.1:1".to_string()])
            .await
            .expect("replace relays should succeed without initialized client");

        assert_eq!(
            manager.relay_statuses().await,
            vec![("ws://127.0.0.1:1".to_string(), "disconnected".to_string())]
        );
    }

    #[tokio::test]
    async fn test_init_with_keys_fails_when_configured_relays_do_not_connect() {
        let mut manager = NostrClientManager::new();
        manager
            .replace_relays(vec!["ws://127.0.0.1:1".to_string()])
            .await
            .expect("store relays");

        let secret_key = sample_secret_key('6');
        let err = manager
            .init_with_keys(&secret_key)
            .await
            .expect_err("initialization should fail when no relay connects");

        assert!(
            err.to_string()
                .contains("No configured Nostr relay connected")
        );
    }

    #[tokio::test]
    async fn test_publish_event_without_relays_returns_fast_error() {
        let mut manager = NostrClientManager::new();
        let secret_key = sample_secret_key('7');
        manager
            .init_with_keys(&secret_key)
            .await
            .expect("initialize client without relays");

        let event = sample_event(&secret_key);
        let err = tokio::time::timeout(Duration::from_millis(100), manager.publish_event(event))
            .await
            .expect("publish should not hang")
            .expect_err("publish should fail without relays");

        assert!(err.to_string().contains("no relays specified"));
    }
}
