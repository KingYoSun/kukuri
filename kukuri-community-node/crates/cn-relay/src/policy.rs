use serde::Deserialize;
use serde_json::Value;
use sqlx::{Pool, Postgres, Row};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct TopicIngestPolicy {
    pub retention_days: Option<i64>,
    pub max_events: Option<i64>,
    pub max_bytes: Option<i64>,
    pub allow_backfill: Option<bool>,
}

impl TopicIngestPolicy {
    pub fn effective_retention_days(&self) -> Option<i64> {
        self.retention_days.filter(|days| *days > 0)
    }

    pub fn effective_max_events(&self) -> Option<i64> {
        self.max_events.filter(|limit| *limit > 0)
    }

    pub fn effective_max_bytes(&self) -> Option<i64> {
        self.max_bytes.filter(|limit| *limit > 0)
    }

    pub fn allows_backfill(&self) -> bool {
        self.allow_backfill.unwrap_or(true)
    }
}

pub(crate) async fn load_topic_ingest_policies(
    pool: &Pool<Postgres>,
    topic_ids: &[String],
) -> Result<HashMap<String, TopicIngestPolicy>, sqlx::Error> {
    if topic_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query(
        "SELECT topic_id, ingest_policy \
         FROM cn_admin.node_subscriptions \
         WHERE topic_id = ANY($1)",
    )
    .bind(topic_ids)
    .fetch_all(pool)
    .await?;

    Ok(parse_policy_rows(rows))
}

pub(crate) async fn load_all_topic_ingest_policies(
    pool: &Pool<Postgres>,
) -> Result<HashMap<String, TopicIngestPolicy>, sqlx::Error> {
    let rows = sqlx::query("SELECT topic_id, ingest_policy FROM cn_admin.node_subscriptions")
        .fetch_all(pool)
        .await?;
    Ok(parse_policy_rows(rows))
}

fn parse_policy_rows(rows: Vec<sqlx::postgres::PgRow>) -> HashMap<String, TopicIngestPolicy> {
    let mut policies = HashMap::new();
    for row in rows {
        let topic_id: String = match row.try_get("topic_id") {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(error = %err, "failed to read topic_id from node_subscriptions");
                continue;
            }
        };
        let raw_policy: Option<Value> = match row.try_get("ingest_policy") {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(topic_id = %topic_id, error = %err, "failed to read ingest_policy");
                continue;
            }
        };

        let Some(raw_policy) = raw_policy else {
            continue;
        };
        match serde_json::from_value::<TopicIngestPolicy>(raw_policy) {
            Ok(policy) => {
                policies.insert(topic_id, policy);
            }
            Err(err) => {
                tracing::warn!(
                    topic_id = %topic_id,
                    error = %err,
                    "ignored invalid ingest_policy payload"
                );
            }
        }
    }
    policies
}
