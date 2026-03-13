use cn_core::service_config::{
    auth_config_from_json, max_concurrent_node_topics_from_json, AuthConfig,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone)]
pub struct RelayRuntimeConfig {
    pub auth: AuthConfig,
    pub limits: RelayLimits,
    pub rate_limit: RelayRateLimit,
    pub node_subscription: RelayNodeSubscription,
    pub retention: RelayRetention,
}

#[derive(Clone)]
pub struct RelayLimits {
    pub max_event_bytes: usize,
    pub max_tags: usize,
}

#[derive(Clone)]
pub struct RelayRateLimit {
    pub enabled: bool,
    pub ws_events_per_minute: u64,
    pub ws_reqs_per_minute: u64,
    pub ws_conns_per_minute: u64,
    pub gossip_msgs_per_minute: u64,
}

#[derive(Clone)]
pub struct RelayNodeSubscription {
    pub max_concurrent_topics: i64,
}

impl RelayRuntimeConfig {
    pub fn from_json(value: &Value) -> Self {
        let auth = auth_config_from_json(value);
        let limits = RelayLimits::from_json(value);
        let rate_limit = RelayRateLimit::from_json(value);
        let node_subscription = RelayNodeSubscription::from_json(value);
        let retention = RelayRetention::from_json(value);
        Self {
            auth,
            limits,
            rate_limit,
            node_subscription,
            retention,
        }
    }
}

#[derive(Deserialize)]
struct LimitsSection {
    max_event_bytes: Option<usize>,
    max_tags: Option<usize>,
}

impl RelayLimits {
    fn from_json(value: &Value) -> Self {
        let limits = value
            .get("limits")
            .and_then(|v| serde_json::from_value::<LimitsSection>(v.clone()).ok());
        RelayLimits {
            max_event_bytes: limits
                .as_ref()
                .and_then(|l| l.max_event_bytes)
                .unwrap_or(32 * 1024),
            max_tags: limits.as_ref().and_then(|l| l.max_tags).unwrap_or(200),
        }
    }
}

#[derive(Deserialize)]
struct RateLimitSection {
    enabled: Option<bool>,
    ws: Option<RateLimitWs>,
    gossip: Option<RateLimitGossip>,
}

#[derive(Deserialize)]
struct RateLimitWs {
    events_per_minute: Option<u64>,
    reqs_per_minute: Option<u64>,
    conns_per_minute: Option<u64>,
}

#[derive(Deserialize)]
struct RateLimitGossip {
    msgs_per_minute: Option<u64>,
}

impl RelayRateLimit {
    fn from_json(value: &Value) -> Self {
        let section = value
            .get("rate_limit")
            .and_then(|v| serde_json::from_value::<RateLimitSection>(v.clone()).ok());
        let ws = section.as_ref().and_then(|s| s.ws.as_ref());
        let gossip = section.as_ref().and_then(|s| s.gossip.as_ref());
        RelayRateLimit {
            enabled: section.as_ref().and_then(|s| s.enabled).unwrap_or(true),
            ws_events_per_minute: ws.and_then(|w| w.events_per_minute).unwrap_or(120),
            ws_reqs_per_minute: ws.and_then(|w| w.reqs_per_minute).unwrap_or(60),
            ws_conns_per_minute: ws.and_then(|w| w.conns_per_minute).unwrap_or(30),
            gossip_msgs_per_minute: gossip.and_then(|g| g.msgs_per_minute).unwrap_or(600),
        }
    }
}

impl RelayNodeSubscription {
    fn from_json(value: &Value) -> Self {
        Self {
            max_concurrent_topics: max_concurrent_node_topics_from_json(value),
        }
    }
}

#[derive(Clone)]
pub struct RelayRetention {
    pub events_days: i64,
    pub tombstone_days: i64,
    pub dedupe_days: i64,
    pub outbox_days: i64,
    pub cleanup_interval_seconds: u64,
}

#[derive(Deserialize)]
struct RetentionSection {
    events_days: Option<i64>,
    tombstone_days: Option<i64>,
    dedupe_days: Option<i64>,
    outbox_days: Option<i64>,
    cleanup_interval_seconds: Option<u64>,
}

impl RelayRetention {
    fn from_json(value: &Value) -> Self {
        let section = value
            .get("retention")
            .and_then(|v| serde_json::from_value::<RetentionSection>(v.clone()).ok());
        RelayRetention {
            events_days: section.as_ref().and_then(|s| s.events_days).unwrap_or(30),
            tombstone_days: section
                .as_ref()
                .and_then(|s| s.tombstone_days)
                .unwrap_or(180),
            dedupe_days: section.as_ref().and_then(|s| s.dedupe_days).unwrap_or(180),
            outbox_days: section.as_ref().and_then(|s| s.outbox_days).unwrap_or(30),
            cleanup_interval_seconds: section
                .as_ref()
                .and_then(|s| s.cleanup_interval_seconds)
                .unwrap_or(3600),
        }
    }
}
