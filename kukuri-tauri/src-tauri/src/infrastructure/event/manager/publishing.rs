use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use nostr_sdk::prelude::*;
use sqlx::Row;
use tokio::sync::RwLock;
use tracing::{error, warn};

use super::EventManager;
use crate::application::shared::mappers::profile_metadata_to_nostr;
use crate::domain::entities::event_gateway::{PrivacyPreferences, ProfileMetadata};
use crate::infrastructure::event::nostr_client_manager::NostrClientManager;
use crate::infrastructure::p2p::GossipService;

fn allow_no_relay_publish(message: &str) -> bool {
    std::env::var("KUKURI_ALLOW_NO_RELAY")
        .map(|value| value == "1")
        .unwrap_or(false)
        || message.contains("no relays specified")
        || message.contains("not connected to any relays")
}

fn metadata_relay_publish_timeout() -> Duration {
    if cfg!(test) {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(3)
    }
}

async fn broadcast_metadata_to_topics(
    gossip: Arc<dyn GossipService>,
    topics: Vec<String>,
    event: Event,
) {
    let mut uniq = HashSet::new();
    for topic in topics {
        if !topic.is_empty() {
            uniq.insert(topic);
        }
    }
    if uniq.is_empty() {
        return;
    }

    let domain_event =
        match crate::application::shared::mappers::nostr_event_to_domain_event(&event) {
            Ok(event) => event,
            Err(err) => {
                error!(
                    "Failed to convert metadata event for P2P broadcast: {}",
                    err
                );
                return;
            }
        };

    for topic in uniq {
        let _ = gossip.join_topic(&topic, vec![]).await;
        if let Err(err) = gossip.broadcast(&topic, &domain_event).await {
            error!("Failed to broadcast metadata to topic {}: {}", topic, err);
        }
    }
}

async fn publish_metadata_to_relays_best_effort(
    client_manager: Arc<RwLock<NostrClientManager>>,
    event: Event,
) {
    let client_manager = client_manager.read().await;
    match tokio::time::timeout(
        metadata_relay_publish_timeout(),
        client_manager.publish_event(event.clone()),
    )
    .await
    {
        Ok(Ok(event_id)) => {
            tracing::debug!(
                target: "event_manager",
                "metadata relay publish completed: {}",
                event_id
            );
        }
        Ok(Err(err)) => {
            let msg = err.to_string();
            if allow_no_relay_publish(&msg) {
                tracing::warn!(
                    target: "event_manager",
                    "metadata relay publish skipped (no relay connected): {msg}"
                );
            } else {
                error!("Failed to publish metadata to relay: {}", err);
            }
        }
        Err(_) => {
            tracing::warn!(
                target: "event_manager",
                "metadata relay publish timed out after {:?}",
                metadata_relay_publish_timeout()
            );
        }
    }
}

fn trim_non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

impl EventManager {
    async fn load_current_profile_metadata(&self) -> Option<Metadata> {
        let pool = self.connection_pool.as_ref()?;
        let public_key = self.get_public_key().await.map(|pk| pk.to_string())?;

        let user_row = match sqlx::query(
            r#"
            SELECT
                name,
                display_name,
                bio,
                avatar_url,
                nip05,
                is_profile_public,
                show_online_status
            FROM users
            WHERE pubkey = ?1
            LIMIT 1
            "#,
        )
        .bind(&public_key)
        .fetch_optional(pool.get_pool())
        .await
        {
            Ok(row) => row,
            Err(err) => {
                warn!(
                    target: "event_manager",
                    error = %err,
                    pubkey = %public_key,
                    "Failed to load current profile from users table"
                );
                None
            }
        };

        let (name, display_name, about, picture, banner, nip05, public_profile, show_online_status) =
            if let Some(row) = user_row {
                (
                    trim_non_empty(row.try_get::<Option<String>, _>("name").ok().flatten()),
                    trim_non_empty(
                        row.try_get::<Option<String>, _>("display_name")
                            .ok()
                            .flatten(),
                    ),
                    trim_non_empty(row.try_get::<Option<String>, _>("bio").ok().flatten()),
                    trim_non_empty(
                        row.try_get::<Option<String>, _>("avatar_url")
                            .ok()
                            .flatten(),
                    ),
                    None,
                    trim_non_empty(row.try_get::<Option<String>, _>("nip05").ok().flatten()),
                    row.try_get::<i64, _>("is_profile_public")
                        .ok()
                        .map(|value| value != 0)
                        .unwrap_or(true),
                    row.try_get::<i64, _>("show_online_status")
                        .ok()
                        .map(|value| value != 0)
                        .unwrap_or(false),
                )
            } else {
                let profile_row = match sqlx::query(
                    r#"
                    SELECT
                        display_name,
                        about,
                        picture_url,
                        banner_url,
                        nip05
                    FROM profiles
                    WHERE public_key = ?1
                    LIMIT 1
                    "#,
                )
                .bind(&public_key)
                .fetch_optional(pool.get_pool())
                .await
                {
                    Ok(row) => row,
                    Err(err) => {
                        warn!(
                            target: "event_manager",
                            error = %err,
                            pubkey = %public_key,
                            "Failed to load current profile from profiles table"
                        );
                        None
                    }
                };
                let row = profile_row?;
                (
                    None,
                    trim_non_empty(
                        row.try_get::<Option<String>, _>("display_name")
                            .ok()
                            .flatten(),
                    ),
                    trim_non_empty(row.try_get::<Option<String>, _>("about").ok().flatten()),
                    trim_non_empty(
                        row.try_get::<Option<String>, _>("picture_url")
                            .ok()
                            .flatten(),
                    ),
                    trim_non_empty(
                        row.try_get::<Option<String>, _>("banner_url")
                            .ok()
                            .flatten(),
                    ),
                    trim_non_empty(row.try_get::<Option<String>, _>("nip05").ok().flatten()),
                    true,
                    false,
                )
            };

        let has_profile_payload = name.is_some()
            || display_name.is_some()
            || about.is_some()
            || picture.is_some()
            || banner.is_some()
            || nip05.is_some()
            || !public_profile
            || show_online_status;
        if !has_profile_payload {
            return None;
        }

        let profile = match ProfileMetadata::new(
            name,
            display_name,
            about,
            picture,
            banner,
            nip05,
            None,
            None,
            None,
            Some(PrivacyPreferences {
                public_profile,
                show_online_status,
            }),
        ) {
            Ok(profile) => profile,
            Err(err) => {
                warn!(
                    target: "event_manager",
                    error = %err,
                    pubkey = %public_key,
                    "Failed to build current profile metadata"
                );
                return None;
            }
        };

        match profile_metadata_to_nostr(&profile) {
            Ok(metadata) => Some(metadata),
            Err(err) => {
                warn!(
                    target: "event_manager",
                    error = %err,
                    pubkey = %public_key,
                    "Failed to convert current profile metadata to nostr metadata"
                );
                None
            }
        }
    }

    async fn publish_current_profile_metadata_to_topic_best_effort(&self, topic_id: &str) {
        let Some(metadata) = self.load_current_profile_metadata().await else {
            return;
        };

        let publisher = self.event_publisher.read().await;
        let metadata_event = match publisher.create_metadata(metadata) {
            Ok(event) => event,
            Err(err) => {
                warn!(
                    target: "event_manager",
                    error = %err,
                    topic_id,
                    "Failed to create current profile metadata event"
                );
                return;
            }
        };
        drop(publisher);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            broadcast_metadata_to_topics(
                gossip,
                vec![topic_id.to_string()],
                metadata_event.clone(),
            )
            .await;
        }

        let client_manager = self.client_manager.clone();
        tokio::spawn(async move {
            publish_metadata_to_relays_best_effort(client_manager, metadata_event).await;
        });
    }

    /// テキストノートを投稿
    pub async fn publish_text_note(&self, content: &str) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_text_note(content, vec![])?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let event_id = match client_manager.publish_event(event.clone()).await {
            Ok(id) => id,
            Err(e) => {
                let msg = e.to_string();
                if allow_no_relay_publish(&msg) {
                    tracing::warn!(
                        target: "event_manager",
                        "publish_event skipped (no relay connected): {msg}"
                    );
                    event.id
                } else {
                    return Err(e);
                }
            }
        };
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topics = self.default_topics_with_user_topic().await;
            if let Err(e) = self.broadcast_to_topics(&gossip, &topics, &event).await {
                error!("Failed to broadcast to P2P (text_note): {}", e);
            }
        }

        Ok(event_id)
    }

    /// トピック投稿を作成・送信
    pub async fn publish_topic_post(
        &self,
        topic_id: &str,
        content: &str,
        reply_to: Option<EventId>,
        scope: Option<&str>,
        epoch: Option<i64>,
    ) -> Result<EventId> {
        self.ensure_initialized().await?;
        self.publish_current_profile_metadata_to_topic_best_effort(topic_id)
            .await;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_topic_post(topic_id, content, reply_to, scope, epoch)?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let event_id = match client_manager.publish_event(event.clone()).await {
            Ok(id) => id,
            Err(e) => {
                let msg = e.to_string();
                let allow_no_relay = allow_no_relay_publish(&msg);

                if allow_no_relay {
                    tracing::warn!(
                        target: "event_manager",
                        "publish_event skipped (no relay connected): {msg}"
                    );
                    event.id
                } else {
                    return Err(e);
                }
            }
        };
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned()
            && let Err(e) = self.broadcast_to_topic(&gossip, topic_id, &event).await
        {
            error!("Failed to broadcast to P2P (topic {}): {}", topic_id, e);
        }

        if let Some(store) = self.event_topic_store.read().await.as_ref().cloned() {
            tracing::debug!(
                target: "event_manager",
                "adding event_topic mapping for {}",
                event.id.to_hex()
            );
            let _ = store.add_event_topic(&event.id.to_string(), topic_id).await;
        }

        Ok(event_id)
    }

    /// リアクションを送信
    pub async fn send_reaction(&self, event_id: &EventId, reaction: &str) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_reaction(event_id, reaction)?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let result_id = client_manager.publish_event(event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topic_list = if let Some(resolved_topics) = self
                .resolve_topics_for_referenced_event(&event_id.to_hex())
                .await
            {
                if resolved_topics.is_empty() {
                    self.default_topics_with_user_topic().await
                } else {
                    let unique: HashSet<_> = resolved_topics.into_iter().collect();
                    unique.into_iter().collect()
                }
            } else {
                self.default_topics_with_user_topic().await
            };

            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast reaction to P2P: {}", e);
            }
        }

        Ok(result_id)
    }

    /// 投稿を再配信 (Repost) する
    pub async fn publish_repost(&self, target: &EventId) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_repost(target)?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let result_id = client_manager.publish_event(event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topic_list = if let Some(resolved_topics) = self
                .resolve_topics_for_referenced_event(&target.to_hex())
                .await
            {
                if resolved_topics.is_empty() {
                    self.default_topics_with_user_topic().await
                } else {
                    let unique: HashSet<_> = resolved_topics.into_iter().collect();
                    unique.into_iter().collect()
                }
            } else {
                self.default_topics_with_user_topic().await
            };

            if let Err(e) = self.broadcast_to_topics(&gossip, &topic_list, &event).await {
                error!("Failed to broadcast repost to P2P: {}", e);
            }
        }

        Ok(result_id)
    }

    /// 指定したイベントを削除するための削除イベントを発行
    pub async fn delete_events(
        &self,
        target_ids: Vec<EventId>,
        reason: Option<String>,
    ) -> Result<EventId> {
        self.ensure_initialized().await?;
        if target_ids.is_empty() {
            return Err(anyhow!("No event IDs provided"));
        }

        let publisher = self.event_publisher.read().await;
        let deletion_event = publisher.create_deletion(target_ids.clone(), reason.as_deref())?;
        drop(publisher);

        let client_manager = self.client_manager.read().await;
        let deletion_event_id = client_manager.publish_event(deletion_event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let mut topics: HashSet<String> = HashSet::new();
            for event_id in &target_ids {
                if let Some(resolved_topics) = self
                    .resolve_topics_for_referenced_event(&event_id.to_hex())
                    .await
                {
                    topics.extend(resolved_topics);
                }
            }

            if topics.is_empty() {
                topics.extend(self.default_topics_with_user_topic().await);
            }

            let topic_list: Vec<String> = topics.into_iter().collect();
            if let Err(e) = self
                .broadcast_to_topics(&gossip, &topic_list, &deletion_event)
                .await
            {
                error!("Failed to broadcast deletion to P2P: {}", e);
            }
        }

        Ok(deletion_event_id)
    }

    /// リポスト（ブースト）を送信
    /// 任意のイベントを発行
    pub async fn publish_event(&self, event: Event) -> Result<EventId> {
        self.ensure_initialized().await?;

        let client_manager = self.client_manager.read().await;
        let event_id = client_manager.publish_event(event.clone()).await?;
        drop(client_manager);

        if let Some(gossip) = self.gossip_service.read().await.as_ref().cloned() {
            let topics = self.default_topics_with_user_topic().await;
            if let Err(e) = self.broadcast_to_topics(&gossip, &topics, &event).await {
                error!("Failed to broadcast generic event to P2P: {}", e);
            }
        }

        Ok(event_id)
    }

    /// メタデータを更新
    pub async fn update_metadata(&self, metadata: Metadata) -> Result<EventId> {
        self.ensure_initialized().await?;

        let publisher = self.event_publisher.read().await;
        let event = publisher.create_metadata(metadata)?;
        drop(publisher);

        let result_id = event.id;
        let gossip = self.gossip_service.read().await.as_ref().cloned();
        let topics = self.default_topics_with_user_topic().await;

        if let Some(gossip) = gossip {
            let event_for_p2p = event.clone();
            let event_for_relay = event.clone();
            let client_manager = self.client_manager.clone();
            tokio::spawn(async move {
                broadcast_metadata_to_topics(gossip, topics, event_for_p2p).await;
            });
            tokio::spawn(async move {
                publish_metadata_to_relays_best_effort(client_manager, event_for_relay).await;
            });
            return Ok(result_id);
        }

        let client_manager = self.client_manager.read().await;
        match tokio::time::timeout(
            metadata_relay_publish_timeout(),
            client_manager.publish_event(event.clone()),
        )
        .await
        {
            Ok(Ok(id)) => Ok(id),
            Ok(Err(err)) => {
                let msg = err.to_string();
                if allow_no_relay_publish(&msg) {
                    tracing::warn!(
                        target: "event_manager",
                        "metadata publish skipped (no relay connected): {msg}"
                    );
                    Ok(result_id)
                } else {
                    Err(err)
                }
            }
            Err(_) => Err(anyhow!(
                "metadata relay publish timed out after {:?}",
                metadata_relay_publish_timeout()
            )),
        }
    }
}
