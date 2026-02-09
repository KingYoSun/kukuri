use super::*;
use cn_core::{nostr, service_config, trust as trust_core};
use nostr_sdk::prelude::Keys;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tokio::sync::OnceCell;
use uuid::Uuid;

static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_tests() -> MutexGuard<'static, ()> {
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:15432/cn".to_string())
}

async fn ensure_migrated(pool: &Pool<Postgres>) {
    MIGRATIONS
        .get_or_init(|| async {
            cn_core::migrations::run(pool)
                .await
                .expect("run migrations");
        })
        .await;
}

fn build_state(pool: Pool<Postgres>, node_keys: Keys, config_json: serde_json::Value) -> AppState {
    AppState {
        pool,
        config: service_config::static_handle(config_json),
        node_keys,
        health_targets: Arc::new(HashMap::new()),
        health_client: reqwest::Client::new(),
    }
}

fn runtime_config_json() -> serde_json::Value {
    json!({
        "enabled": true,
        "consumer": {
            "batch_size": 200,
            "poll_interval_seconds": 1
        },
        "report_based": {
            "window_days": 30,
            "report_weight": 1.0,
            "label_weight": 1.0,
            "score_normalization": 10.0
        },
        "communication_density": {
            "window_days": 30,
            "score_normalization": 10.0,
            "interaction_weights": {
                "1": 1.0
            }
        },
        "attestation": {
            "exp_seconds": 3600
        },
        "jobs": {
            "schedule_poll_seconds": 1,
            "report_based_interval_seconds": 60,
            "communication_interval_seconds": 90
        }
    })
}

async fn insert_event(pool: &Pool<Postgres>, topic_id: &str, event: &nostr::RawEvent) {
    sqlx::query(
        "INSERT INTO cn_relay.events \
         (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, ingested_at, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), FALSE, FALSE, TRUE, NULL, NULL, NULL) \
         ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(&event.id)
    .bind(&event.pubkey)
    .bind(event.kind as i32)
    .bind(event.created_at)
    .bind(serde_json::to_value(&event.tags).expect("serialize tags"))
    .bind(&event.content)
    .bind(&event.sig)
    .bind(serde_json::to_value(event).expect("serialize raw event"))
    .execute(pool)
    .await
    .expect("insert relay event");

    sqlx::query(
        "INSERT INTO cn_relay.event_topics (event_id, topic_id) \
         VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(&event.id)
    .bind(topic_id)
    .execute(pool)
    .await
    .expect("insert event topic");
}

async fn insert_outbox_row(pool: &Pool<Postgres>, topic_id: &str, event: &nostr::RawEvent) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO cn_relay.events_outbox \
         (op, event_id, topic_id, kind, created_at, ingested_at, effective_key, reason) \
         VALUES ('upsert', $1, $2, $3, $4, NOW(), NULL, 'integration-test') \
         RETURNING seq",
    )
    .bind(&event.id)
    .bind(topic_id)
    .bind(event.kind as i32)
    .bind(event.created_at)
    .fetch_one(pool)
    .await
    .expect("insert outbox row")
}

async fn outbox_row_by_seq(pool: &Pool<Postgres>, seq: i64) -> OutboxRow {
    let row = sqlx::query(
        "SELECT seq, op, event_id, topic_id, kind, created_at \
         FROM cn_relay.events_outbox \
         WHERE seq = $1",
    )
    .bind(seq)
    .fetch_one(pool)
    .await
    .expect("load outbox row");

    OutboxRow {
        seq: row.try_get("seq").expect("seq"),
        op: row.try_get("op").expect("op"),
        event_id: row.try_get("event_id").expect("event_id"),
        topic_id: row.try_get("topic_id").expect("topic_id"),
        kind: row.try_get("kind").expect("kind"),
        created_at: row.try_get("created_at").expect("created_at"),
    }
}

async fn load_attestation_event(pool: &Pool<Postgres>, attestation_id: &str) -> nostr::RawEvent {
    let event_json: serde_json::Value = sqlx::query_scalar(
        "SELECT event_json FROM cn_trust.attestations WHERE attestation_id = $1",
    )
    .bind(attestation_id)
    .fetch_one(pool)
    .await
    .expect("attestation event json");
    nostr::parse_event(&event_json).expect("parse attestation event")
}

async fn cleanup_artifacts(pool: &Pool<Postgres>, event_ids: &[String], pubkeys: &[String]) {
    let event_refs: Vec<&str> = event_ids.iter().map(String::as_str).collect();
    let pubkey_refs: Vec<&str> = pubkeys.iter().map(String::as_str).collect();
    let subject_refs: Vec<String> = pubkeys
        .iter()
        .map(|pubkey| format!("pubkey:{pubkey}"))
        .collect();
    let subject_ref_slices: Vec<&str> = subject_refs.iter().map(String::as_str).collect();
    let job_types = vec![JOB_REPORT_BASED, JOB_COMMUNICATION];

    sqlx::query("DELETE FROM cn_trust.jobs WHERE job_type = ANY($1)")
        .bind(&job_types)
        .execute(pool)
        .await
        .expect("cleanup trust jobs");
    sqlx::query("DELETE FROM cn_trust.job_schedules WHERE job_type = ANY($1)")
        .bind(&job_types)
        .execute(pool)
        .await
        .expect("cleanup trust job schedules");

    if !event_refs.is_empty() {
        sqlx::query("DELETE FROM cn_trust.interactions WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup trust interactions by event");
        sqlx::query("DELETE FROM cn_trust.report_events WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup trust report events by event");
        sqlx::query("DELETE FROM cn_relay.events_outbox WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup outbox");
        sqlx::query("DELETE FROM cn_relay.event_topics WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup event topics");
        sqlx::query("DELETE FROM cn_relay.events WHERE event_id = ANY($1)")
            .bind(&event_refs)
            .execute(pool)
            .await
            .expect("cleanup relay events");
    }

    if !pubkey_refs.is_empty() {
        sqlx::query(
            "DELETE FROM cn_trust.interactions \
             WHERE actor_pubkey = ANY($1) OR target_pubkey = ANY($1)",
        )
        .bind(&pubkey_refs)
        .execute(pool)
        .await
        .expect("cleanup trust interactions by pubkey");
        sqlx::query(
            "DELETE FROM cn_trust.report_events \
             WHERE subject_pubkey = ANY($1) OR reporter_pubkey = ANY($1)",
        )
        .bind(&pubkey_refs)
        .execute(pool)
        .await
        .expect("cleanup trust report events by pubkey");
        sqlx::query("DELETE FROM cn_trust.report_scores WHERE subject_pubkey = ANY($1)")
            .bind(&pubkey_refs)
            .execute(pool)
            .await
            .expect("cleanup trust report scores");
        sqlx::query("DELETE FROM cn_trust.communication_scores WHERE subject_pubkey = ANY($1)")
            .bind(&pubkey_refs)
            .execute(pool)
            .await
            .expect("cleanup trust communication scores");
    }

    if !subject_ref_slices.is_empty() {
        sqlx::query("DELETE FROM cn_trust.attestations WHERE subject = ANY($1)")
            .bind(&subject_ref_slices)
            .execute(pool)
            .await
            .expect("cleanup trust attestations");
    }
}

#[tokio::test(flavor = "current_thread")]
async fn report_and_interaction_ingest_updates_scores_attestations_and_jobs() {
    let _guard = lock_tests();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url())
        .await
        .expect("connect database");
    ensure_migrated(&pool).await;
    ensure_graph(&pool).await.expect("ensure age graph");

    let topic_id = format!("kukuri:trust-it:{}", Uuid::new_v4());
    let node_keys = Keys::generate();
    let reporter_keys = Keys::generate();
    let actor_keys = Keys::generate();
    let subject_keys = Keys::generate();

    let subject_event = nostr::build_signed_event(
        &subject_keys,
        1,
        vec![vec!["t".to_string(), topic_id.clone()]],
        "subject-anchor".to_string(),
    )
    .expect("build subject event");
    let report_event = nostr::build_signed_event(
        &reporter_keys,
        39005,
        vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["target".to_string(), format!("event:{}", subject_event.id)],
            vec!["reason".to_string(), "spam".to_string()],
        ],
        "report".to_string(),
    )
    .expect("build report event");
    let interaction_event = nostr::build_signed_event(
        &actor_keys,
        1,
        vec![
            vec!["t".to_string(), topic_id.clone()],
            vec!["scope".to_string(), "public".to_string()],
            vec!["p".to_string(), subject_event.pubkey.clone()],
        ],
        "interaction".to_string(),
    )
    .expect("build interaction event");

    let tracked_event_ids = vec![
        subject_event.id.clone(),
        report_event.id.clone(),
        interaction_event.id.clone(),
    ];
    let tracked_pubkeys = vec![
        subject_event.pubkey.clone(),
        interaction_event.pubkey.clone(),
    ];
    cleanup_artifacts(&pool, &tracked_event_ids, &tracked_pubkeys).await;

    insert_event(&pool, &topic_id, &subject_event).await;
    insert_event(&pool, &topic_id, &report_event).await;
    insert_event(&pool, &topic_id, &interaction_event).await;
    let report_seq = insert_outbox_row(&pool, &topic_id, &report_event).await;
    let interaction_seq = insert_outbox_row(&pool, &topic_id, &interaction_event).await;

    let config_json = runtime_config_json();
    let runtime = config::TrustRuntimeConfig::from_json(&config_json);
    let state = build_state(pool.clone(), node_keys, config_json);

    let report_row = outbox_row_by_seq(&pool, report_seq).await;
    handle_outbox_row(&state, &runtime, &report_row)
        .await
        .expect("handle report outbox row");
    let interaction_row = outbox_row_by_seq(&pool, interaction_seq).await;
    handle_outbox_row(&state, &runtime, &interaction_row)
        .await
        .expect("handle interaction outbox row");

    let report_event_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_trust.report_events WHERE event_id = $1")
            .bind(&report_event.id)
            .fetch_one(&pool)
            .await
            .expect("count report events");
    assert_eq!(report_event_count, 1);

    let interaction_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_trust.interactions WHERE event_id = $1")
            .bind(&interaction_event.id)
            .fetch_one(&pool)
            .await
            .expect("count interactions");
    assert_eq!(interaction_count, 1);

    let report_score_row = sqlx::query(
        "SELECT score, report_count, label_count, attestation_id \
         FROM cn_trust.report_scores \
         WHERE subject_pubkey = $1",
    )
    .bind(&subject_event.pubkey)
    .fetch_one(&pool)
    .await
    .expect("load report score");
    let report_score: f64 = report_score_row.try_get("score").expect("report score");
    let report_count: i64 = report_score_row
        .try_get("report_count")
        .expect("report count");
    let label_count: i64 = report_score_row
        .try_get("label_count")
        .expect("label count");
    let report_attestation_id: Option<String> = report_score_row
        .try_get("attestation_id")
        .expect("report attestation id");
    assert!(report_score > 0.0);
    assert_eq!(report_count, 1);
    assert_eq!(label_count, 0);
    let report_attestation_id = report_attestation_id.expect("report attestation id");

    let report_attestation = load_attestation_event(&pool, &report_attestation_id).await;
    assert_eq!(report_attestation.kind, 39010);
    assert_eq!(
        report_attestation.first_tag_value("claim").as_deref(),
        Some(trust_core::CLAIM_REPORT_BASED)
    );
    assert_eq!(
        report_attestation.first_tag_value("sub").as_deref(),
        Some("pubkey")
    );

    let communication_score_row = sqlx::query(
        "SELECT score, interaction_count, peer_count, attestation_id \
         FROM cn_trust.communication_scores \
         WHERE subject_pubkey = $1",
    )
    .bind(&subject_event.pubkey)
    .fetch_one(&pool)
    .await
    .expect("load communication score");
    let communication_score: f64 = communication_score_row
        .try_get("score")
        .expect("communication score");
    let communication_edges: i64 = communication_score_row
        .try_get("interaction_count")
        .expect("interaction_count");
    let communication_peers: i64 = communication_score_row
        .try_get("peer_count")
        .expect("peer_count");
    let communication_attestation_id: Option<String> = communication_score_row
        .try_get("attestation_id")
        .expect("communication attestation id");
    assert!(communication_score > 0.0);
    assert_eq!(communication_edges, 1);
    assert_eq!(communication_peers, 1);
    let communication_attestation_id =
        communication_attestation_id.expect("communication attestation id");

    let communication_attestation =
        load_attestation_event(&pool, &communication_attestation_id).await;
    assert_eq!(communication_attestation.kind, 39010);
    assert_eq!(
        communication_attestation
            .first_tag_value("claim")
            .as_deref(),
        Some(trust_core::CLAIM_COMMUNICATION_DENSITY)
    );
    assert_eq!(
        communication_attestation.first_tag_value("sub").as_deref(),
        Some("pubkey")
    );

    ensure_job_schedules(&pool, &runtime)
        .await
        .expect("ensure job schedules");

    let schedule_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_trust.job_schedules \
         WHERE job_type IN ($1, $2)",
    )
    .bind(JOB_REPORT_BASED)
    .bind(JOB_COMMUNICATION)
    .fetch_one(&pool)
    .await
    .expect("count schedules");
    assert_eq!(schedule_count, 2);

    sqlx::query(
        "UPDATE cn_trust.job_schedules \
         SET next_run_at = NOW() - INTERVAL '5 seconds' \
         WHERE job_type IN ($1, $2)",
    )
    .bind(JOB_REPORT_BASED)
    .bind(JOB_COMMUNICATION)
    .execute(&pool)
    .await
    .expect("force schedules due");

    let mut due = load_due_schedules(&pool).await.expect("load due schedules");
    due.retain(|schedule| {
        schedule.job_type == JOB_REPORT_BASED || schedule.job_type == JOB_COMMUNICATION
    });
    assert_eq!(due.len(), 2);

    for schedule in &due {
        enqueue_scheduled_job(&pool, schedule)
            .await
            .expect("enqueue schedule");
    }

    let next_run_future_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_trust.job_schedules \
         WHERE job_type IN ($1, $2) \
           AND next_run_at > NOW()",
    )
    .bind(JOB_REPORT_BASED)
    .bind(JOB_COMMUNICATION)
    .fetch_one(&pool)
    .await
    .expect("count future schedules");
    assert_eq!(next_run_future_count, 2);

    let mut finalized_job_types = HashSet::new();
    while let Some(job) = claim_job(&pool).await.expect("claim job") {
        let result = process_job(&state, &runtime, &job).await;
        finalize_job(
            &pool,
            &job,
            result.as_ref().map(|_| ()).map_err(|err| err.to_string()),
        )
        .await
        .expect("finalize job");
        if let Err(err) = result {
            panic!("job {} failed: {}", job.job_type, err);
        }
        finalized_job_types.insert(job.job_type);
        if finalized_job_types.len() >= 2 {
            break;
        }
    }

    assert!(finalized_job_types.contains(JOB_REPORT_BASED));
    assert!(finalized_job_types.contains(JOB_COMMUNICATION));

    let job_rows = sqlx::query(
        "SELECT job_type, status, total_targets, processed_targets, error_message \
         FROM cn_trust.jobs \
         WHERE job_type IN ($1, $2)",
    )
    .bind(JOB_REPORT_BASED)
    .bind(JOB_COMMUNICATION)
    .fetch_all(&pool)
    .await
    .expect("load jobs");
    assert_eq!(job_rows.len(), 2);
    for row in job_rows {
        let status: String = row.try_get("status").expect("job status");
        let total_targets: Option<i64> = row.try_get("total_targets").expect("total_targets");
        let processed_targets: i64 = row.try_get("processed_targets").expect("processed_targets");
        let error_message: Option<String> = row.try_get("error_message").expect("error_message");
        assert_eq!(status, "succeeded");
        assert!(total_targets.unwrap_or(0) >= 1);
        assert!(processed_targets >= 1);
        assert!(error_message.is_none());
    }

    cleanup_artifacts(&pool, &tracked_event_ids, &tracked_pubkeys).await;
}
