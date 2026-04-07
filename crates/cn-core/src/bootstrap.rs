use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::postgres::PgPool;
use sqlx::{Executor, Row};

use crate::config::BOOTSTRAP_PEER_REGISTRATION_TTL_SECONDS;
use crate::database::ensure_active_subscriber;
use crate::models::{
    BootstrapHeartbeatResponse, CommunityNodeBootstrapNode, CommunityNodeResolvedUrls,
    CommunityNodeSeedPeer, normalize_seed_peers,
};
use crate::normalize::{normalize_http_url, normalize_pubkey};

pub async fn refresh_bootstrap_peer_registration(
    pool: &PgPool,
    pubkey: &str,
    endpoint_id: &str,
    addr_hint: Option<&str>,
) -> Result<BootstrapHeartbeatResponse> {
    let pubkey = normalize_pubkey(pubkey)?;
    let mut tx = pool.begin().await?;
    prune_expired_bootstrap_peer_registrations(&mut *tx).await?;
    ensure_active_subscriber(&mut *tx, pubkey.as_str()).await?;
    let now = Utc::now();
    let seed_peer = CommunityNodeSeedPeer::new(endpoint_id, addr_hint.map(str::to_string))?;
    let expires_at =
        upsert_bootstrap_peer_registration(&mut *tx, pubkey.as_str(), &seed_peer, now).await?;
    tx.commit().await?;
    Ok(BootstrapHeartbeatResponse {
        expires_at: expires_at.timestamp(),
    })
}

pub async fn load_bootstrap_nodes(
    pool: &PgPool,
    self_node: Option<CommunityNodeBootstrapNode>,
) -> Result<Vec<CommunityNodeBootstrapNode>> {
    let rows = sqlx::query(
        "SELECT base_url, public_base_url, connectivity_urls
         FROM cn_bootstrap.bootstrap_nodes
         WHERE is_active = TRUE
         ORDER BY base_url ASC",
    )
    .fetch_all(pool)
    .await?;
    let mut nodes = BTreeMap::new();
    if let Some(node) = self_node {
        nodes.insert(node.base_url.clone(), node);
    }
    for row in rows {
        let base_url: String = row.try_get("base_url")?;
        let public_base_url: String = row.try_get("public_base_url")?;
        let connectivity_urls: Value = row.try_get("connectivity_urls")?;
        let connectivity_urls = serde_json::from_value::<Vec<String>>(connectivity_urls)?;
        let node = CommunityNodeBootstrapNode {
            base_url: normalize_http_url(base_url.as_str())?,
            resolved_urls: CommunityNodeResolvedUrls::new(
                public_base_url,
                connectivity_urls,
                Vec::new(),
            )?,
        };
        nodes.insert(node.base_url.clone(), node);
    }
    Ok(nodes.into_values().collect())
}

pub async fn upsert_bootstrap_node(pool: &PgPool, node: &CommunityNodeBootstrapNode) -> Result<()> {
    let relay_urls = serde_json::to_value(&node.resolved_urls.connectivity_urls)?;
    sqlx::query(
        "INSERT INTO cn_bootstrap.bootstrap_nodes
            (base_url, public_base_url, connectivity_urls, is_active)
         VALUES ($1, $2, $3, TRUE)
         ON CONFLICT (base_url) DO UPDATE
         SET public_base_url = EXCLUDED.public_base_url,
             connectivity_urls = EXCLUDED.connectivity_urls,
             is_active = TRUE,
             updated_at = NOW()",
    )
    .bind(&node.base_url)
    .bind(&node.resolved_urls.public_base_url)
    .bind(relay_urls)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn load_bootstrap_seed_peers(
    pool: &PgPool,
    exclude_pubkey: Option<&str>,
    exclude_endpoint_id: Option<&str>,
) -> Result<Vec<CommunityNodeSeedPeer>> {
    let exclude_pubkey = exclude_pubkey.map(normalize_pubkey).transpose()?;
    let exclude_endpoint_id = exclude_endpoint_id
        .map(|value| CommunityNodeSeedPeer::new(value, None))
        .transpose()?
        .map(|seed_peer| seed_peer.endpoint_id);
    prune_expired_bootstrap_peer_registrations(pool).await?;
    let rows = sqlx::query(
        "SELECT peers.endpoint_id, peers.addr_hint
         FROM cn_bootstrap.peer_registrations peers
         JOIN cn_user.subscriber_accounts subscribers
           ON subscribers.subscriber_pubkey = peers.subscriber_pubkey
         WHERE subscribers.status = 'active'
           AND peers.expires_at > NOW()
           AND (
             $1::TEXT IS NULL
             OR ($2::TEXT IS NULL AND peers.subscriber_pubkey <> $1)
             OR ($2::TEXT IS NOT NULL AND (peers.subscriber_pubkey <> $1 OR peers.endpoint_id <> $2))
           )
         ORDER BY peers.last_seen_at DESC, peers.subscriber_pubkey ASC, peers.endpoint_id ASC",
    )
    .bind(exclude_pubkey.as_deref())
    .bind(exclude_endpoint_id.as_deref())
    .fetch_all(pool)
    .await?;
    let mut seed_peers = Vec::with_capacity(rows.len());
    for row in rows {
        let endpoint_id: String = row.try_get("endpoint_id")?;
        let addr_hint: Option<String> = row.try_get("addr_hint")?;
        seed_peers.push(CommunityNodeSeedPeer::new(endpoint_id, addr_hint)?);
    }
    normalize_seed_peers(seed_peers)
}

pub(crate) async fn upsert_bootstrap_peer_registration<'e, E>(
    executor: E,
    pubkey: &str,
    seed_peer: &CommunityNodeSeedPeer,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    let seed_peer =
        CommunityNodeSeedPeer::new(seed_peer.endpoint_id.clone(), seed_peer.addr_hint.clone())?;
    let expires_at = now + Duration::seconds(BOOTSTRAP_PEER_REGISTRATION_TTL_SECONDS);
    sqlx::query(
        "INSERT INTO cn_bootstrap.peer_registrations
            (subscriber_pubkey, endpoint_id, addr_hint, last_seen_at, expires_at)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (subscriber_pubkey, endpoint_id) DO UPDATE
         SET addr_hint = EXCLUDED.addr_hint,
             last_seen_at = EXCLUDED.last_seen_at,
             expires_at = EXCLUDED.expires_at",
    )
    .bind(pubkey)
    .bind(seed_peer.endpoint_id)
    .bind(seed_peer.addr_hint)
    .bind(now)
    .bind(expires_at)
    .execute(executor)
    .await?;
    Ok(expires_at)
}

pub(crate) async fn prune_expired_bootstrap_peer_registrations<'e, E>(executor: E) -> Result<()>
where
    E: Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "DELETE FROM cn_bootstrap.peer_registrations
         WHERE expires_at <= NOW()",
    )
    .execute(executor)
    .await?;
    Ok(())
}
