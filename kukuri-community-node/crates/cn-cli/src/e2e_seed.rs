use anyhow::{anyhow, Result};
use chrono::TimeZone;
use cn_core::{auth, config as env_config, db, meili, moderation, nostr, trust as trust_core};
use nostr_sdk::prelude::{EventBuilder, Keys, Kind, SecretKey, Tag, TagKind, Timestamp};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::types::Json;
use sqlx::{Pool, Postgres, Row};
use std::collections::{BTreeMap, BTreeSet, HashSet};

const SEED_TAG_KEY: &str = "seed";
const SEED_TAG_VALUE: &str = "community-node-e2e";
const SEED_SOURCE: &str = "e2e_seed";

const SEED_TOPIC_ALPHA: &str = "kukuri:e2e-alpha";
const SEED_TOPIC_BETA: &str = "kukuri:e2e-beta";
const SEED_TOPICS: [&str; 2] = [SEED_TOPIC_ALPHA, SEED_TOPIC_BETA];

const SEED_SUBSCRIBER_SECRET: &str =
    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const SEED_AUTHOR_A_SECRET: &str =
    "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";
const SEED_AUTHOR_B_SECRET: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SEED_NODE_SECRET: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Clone)]
struct SeedActor {
    name: &'static str,
    keys: Keys,
    pubkey: String,
}

impl SeedActor {
    fn new(name: &'static str, secret_hex: &str) -> Result<Self> {
        let secret = SecretKey::from_hex(secret_hex)
            .map_err(|err| anyhow!("invalid seed secret for {name}: {err}"))?;
        let keys = Keys::new(secret);
        Ok(Self {
            name,
            pubkey: keys.public_key().to_hex(),
            keys,
        })
    }
}

struct SeedContext {
    now: i64,
    subscriber: SeedActor,
    author_a: SeedActor,
    author_b: SeedActor,
    node: SeedActor,
}

impl SeedContext {
    fn new() -> Result<Self> {
        let now = auth::unix_seconds()? as i64;
        Ok(Self {
            now,
            subscriber: SeedActor::new("subscriber", SEED_SUBSCRIBER_SECRET)?,
            author_a: SeedActor::new("author_a", SEED_AUTHOR_A_SECRET)?,
            author_b: SeedActor::new("author_b", SEED_AUTHOR_B_SECRET)?,
            node: SeedActor::new("node", SEED_NODE_SECRET)?,
        })
    }

    fn seed_users(&self) -> Vec<String> {
        vec![self.subscriber.pubkey.clone()]
    }

    fn seed_subjects(&self) -> Vec<String> {
        vec![self.author_a.pubkey.clone(), self.author_b.pubkey.clone()]
    }
}

#[derive(Clone)]
struct SeedEvent {
    raw: nostr::RawEvent,
    topic_id: String,
}

#[derive(Serialize)]
struct IndexDocument {
    event_id: String,
    topic_id: String,
    kind: i32,
    author: String,
    created_at: i64,
    title: String,
    summary: String,
    content: String,
    tags: Vec<String>,
}

#[derive(Serialize)]
pub struct SeedPostSummary {
    pub event_id: String,
    pub author_pubkey: String,
    pub topic_id: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct SeedSummary {
    pub label_target: String,
    pub label_target_event_id: String,
    pub label: String,
    pub label_confidence: f64,
    pub trust_subject_pubkey: String,
    pub trust_report_score: f64,
    pub trust_density_score: f64,
    pub post: SeedPostSummary,
}

pub async fn seed() -> Result<SeedSummary> {
    let database_url = env_config::required_env("DATABASE_URL")?;
    let meili_url = env_config::required_env("MEILI_URL")?;
    let meili_master_key = std::env::var("MEILI_MASTER_KEY").ok();

    let pool = db::connect(&database_url).await?;
    let meili = meili::MeiliClient::new(meili_url, meili_master_key)?;
    let ctx = SeedContext::new()?;

    cleanup_with_clients(&pool, &meili, &ctx).await?;
    let summary = seed_with_clients(&pool, &meili, &ctx).await?;
    Ok(summary)
}

pub async fn cleanup() -> Result<()> {
    let database_url = env_config::required_env("DATABASE_URL")?;
    let meili_url = env_config::required_env("MEILI_URL")?;
    let meili_master_key = std::env::var("MEILI_MASTER_KEY").ok();

    let pool = db::connect(&database_url).await?;
    let meili = meili::MeiliClient::new(meili_url, meili_master_key)?;
    let ctx = SeedContext::new()?;
    cleanup_with_clients(&pool, &meili, &ctx).await?;
    Ok(())
}

async fn seed_with_clients(
    pool: &Pool<Postgres>,
    meili: &meili::MeiliClient,
    ctx: &SeedContext,
) -> Result<SeedSummary> {
    upsert_subscriber(pool, &ctx.subscriber.pubkey).await?;
    for topic_id in SEED_TOPICS {
        upsert_topic_subscription(pool, topic_id, &ctx.subscriber.pubkey).await?;
        upsert_node_subscription(pool, topic_id).await?;
        upsert_topic_service(pool, topic_id, "bootstrap", "public").await?;
        upsert_topic_service(pool, topic_id, "relay", "public").await?;
    }

    let events = build_seed_events(ctx)?;
    for event in &events {
        insert_relay_event(pool, &event.raw).await?;
        insert_event_topic(pool, &event.raw.id, &event.topic_id).await?;
    }

    seed_meili_documents(meili, &events).await?;

    let label_target = format!("event:{}", events[0].raw.id);
    let label_input = moderation::LabelInput {
        target: label_target.clone(),
        label: "safe".to_string(),
        confidence: Some(0.82),
        exp: ctx.now + 7 * 86400,
        policy_url: "https://example.com/policy".to_string(),
        policy_ref: "e2e".to_string(),
        topic_id: Some(SEED_TOPIC_ALPHA.to_string()),
    };
    let label_event = moderation::build_label_event(&ctx.node.keys, &label_input)?;
    insert_label(pool, &label_event, &label_input, ctx.now).await?;

    let subject_pubkey = ctx.author_a.pubkey.clone();
    let subject = format!("pubkey:{subject_pubkey}");
    let report_exp = ctx.now + 86400;
    let report_score = 0.35_f64;
    let report_context = json!({
        "method": trust_core::METHOD_REPORT_BASED,
        "seed": SEED_TAG_VALUE
    });
    let report_value = json!({
        "score": 0.35,
        "reports": 2,
        "labels": 1
    });
    let report_attestation = trust_core::build_attestation_event(
        &ctx.node.keys,
        &trust_core::AttestationInput {
            subject: subject.clone(),
            claim: trust_core::CLAIM_REPORT_BASED.to_string(),
            score: report_score,
            value: report_value.clone(),
            evidence: Vec::new(),
            context: report_context.clone(),
            exp: report_exp,
            topic_id: Some(SEED_TOPIC_ALPHA.to_string()),
        },
    )?;
    insert_attestation(
        pool,
        &report_attestation,
        &subject,
        trust_core::CLAIM_REPORT_BASED,
        report_score,
        report_exp,
        &report_value,
        &report_context,
        Some(SEED_TOPIC_ALPHA),
    )
    .await?;
    upsert_report_score(pool, &subject_pubkey, report_attestation.id.clone(), report_exp).await?;

    let comm_exp = ctx.now + 86400;
    let comm_score = 0.78_f64;
    let comm_context = json!({
        "method": trust_core::METHOD_COMMUNICATION_DENSITY,
        "seed": SEED_TAG_VALUE
    });
    let comm_value = json!({
        "score": 0.78,
        "interactions": 4,
        "peers": 2
    });
    let comm_attestation = trust_core::build_attestation_event(
        &ctx.node.keys,
        &trust_core::AttestationInput {
            subject: subject.clone(),
            claim: trust_core::CLAIM_COMMUNICATION_DENSITY.to_string(),
            score: comm_score,
            value: comm_value.clone(),
            evidence: Vec::new(),
            context: comm_context.clone(),
            exp: comm_exp,
            topic_id: None,
        },
    )?;
    insert_attestation(
        pool,
        &comm_attestation,
        &subject,
        trust_core::CLAIM_COMMUNICATION_DENSITY,
        comm_score,
        comm_exp,
        &comm_value,
        &comm_context,
        None,
    )
    .await?;
    upsert_communication_score(pool, &subject_pubkey, comm_attestation.id.clone(), comm_exp).await?;

    let primary_event = events
        .first()
        .ok_or_else(|| anyhow!("seed events are empty"))?;
    let summary = SeedSummary {
        label_target: label_target.clone(),
        label_target_event_id: primary_event.raw.id.clone(),
        label: label_input.label.clone(),
        label_confidence: label_input.confidence.unwrap_or_default(),
        trust_subject_pubkey: subject_pubkey.clone(),
        trust_report_score: report_score,
        trust_density_score: comm_score,
        post: SeedPostSummary {
            event_id: primary_event.raw.id.clone(),
            author_pubkey: primary_event.raw.pubkey.clone(),
            topic_id: primary_event.topic_id.clone(),
            content: primary_event.raw.content.clone(),
            created_at: primary_event.raw.created_at,
        },
    };

    tracing::info!(
        subscriber_pubkey = %ctx.subscriber.pubkey,
        author_pubkey = %ctx.author_a.pubkey,
        "community node e2e seed applied"
    );
    Ok(summary)
}

async fn cleanup_with_clients(
    pool: &Pool<Postgres>,
    meili: &meili::MeiliClient,
    ctx: &SeedContext,
) -> Result<()> {
    let seed_tag = Json(json!([[SEED_TAG_KEY, SEED_TAG_VALUE]]));
    let rows = sqlx::query(
        "SELECT e.event_id, t.topic_id          FROM cn_relay.events e          JOIN cn_relay.event_topics t            ON e.event_id = t.event_id          WHERE e.tags @> $1",
    )
    .bind(seed_tag)
    .fetch_all(pool)
    .await?;

    let mut ids_by_topic: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut event_ids = BTreeSet::new();
    for row in rows {
        let event_id: String = row.try_get("event_id")?;
        let topic_id: String = row.try_get("topic_id")?;
        ids_by_topic
            .entry(topic_id)
            .or_default()
            .push(event_id.clone());
        event_ids.insert(event_id);
    }

    for ids in ids_by_topic.values_mut() {
        ids.sort();
        ids.dedup();
    }

    for (topic_id, ids) in &ids_by_topic {
        let uid = meili::topic_index_uid(topic_id);
        if let Err(err) = meili.delete_documents(&uid, ids).await {
            let message = err.to_string();
            if !message.contains("404") {
                return Err(err);
            }
        }
    }

    let event_ids: Vec<String> = event_ids.into_iter().collect();
    if !event_ids.is_empty() {
        sqlx::query("DELETE FROM cn_relay.events_outbox WHERE event_id = ANY($1)")
            .bind(&event_ids)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM cn_relay.event_topics WHERE event_id = ANY($1)")
            .bind(&event_ids)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM cn_relay.events WHERE event_id = ANY($1)")
            .bind(&event_ids)
            .execute(pool)
            .await?;
    }

    sqlx::query("DELETE FROM cn_moderation.labels WHERE source = $1")
        .bind(SEED_SOURCE)
        .execute(pool)
        .await?;

    let subjects = ctx.seed_subjects();
    sqlx::query("DELETE FROM cn_trust.report_scores WHERE subject_pubkey = ANY($1)")
        .bind(&subjects)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM cn_trust.communication_scores WHERE subject_pubkey = ANY($1)")
        .bind(&subjects)
        .execute(pool)
        .await?;

    let attestation_marker = Json(json!({ "seed": SEED_TAG_VALUE }));
    sqlx::query("DELETE FROM cn_trust.attestations WHERE context_json @> $1")
        .bind(attestation_marker)
        .execute(pool)
        .await?;

    let seed_users = ctx.seed_users();
    let seed_topics = SEED_TOPICS.iter().map(|topic| topic.to_string()).collect::<Vec<_>>();
    sqlx::query(
        "DELETE FROM cn_user.topic_subscriptions WHERE subscriber_pubkey = ANY($1) AND topic_id = ANY($2)",
    )
    .bind(&seed_users)
    .bind(&seed_topics)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM cn_user.subscriber_accounts WHERE subscriber_pubkey = ANY($1)")
        .bind(&seed_users)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM cn_admin.node_subscriptions WHERE topic_id = ANY($1)")
        .bind(&seed_topics)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM cn_admin.topic_services WHERE topic_id = ANY($1)")
        .bind(&seed_topics)
        .execute(pool)
        .await?;

    tracing::info!("community node e2e seed cleanup completed");
    Ok(())
}

fn build_seed_events(ctx: &SeedContext) -> Result<Vec<SeedEvent>> {
    let base = ctx.now.saturating_sub(3600);
    let post_alpha = build_post(
        &ctx.author_a.keys,
        SEED_TOPIC_ALPHA,
        "Alpha Seed Post",
        "E2E seed alpha post",
        base,
    )?;
    let post_alpha_extra_one = build_post(
        &ctx.author_b.keys,
        SEED_TOPIC_ALPHA,
        "Alpha Extra One",
        "E2E seed alpha extra one",
        base + 300,
    )?;
    let post_alpha_extra_two = build_post(
        &ctx.author_a.keys,
        SEED_TOPIC_ALPHA,
        "Alpha Extra Two",
        "E2E seed alpha extra two",
        base + 420,
    )?;
    let post_alpha_extra_three = build_post(
        &ctx.author_b.keys,
        SEED_TOPIC_ALPHA,
        "Alpha Extra Three",
        "E2E seed alpha extra three",
        base + 480,
    )?;
    let post_alpha_extra_four = build_post(
        &ctx.author_a.keys,
        SEED_TOPIC_ALPHA,
        "Alpha Extra Four",
        "E2E seed alpha extra four",
        base + 540,
    )?;
    let post_alpha_follow = build_post(
        &ctx.author_b.keys,
        SEED_TOPIC_ALPHA,
        "Alpha Follow-up",
        "E2E seed alpha follow-up",
        base + 600,
    )?;
    let post_beta = build_post(
        &ctx.author_a.keys,
        SEED_TOPIC_BETA,
        "Beta Seed Post",
        "E2E seed beta post",
        base + 1200,
    )?;

    let reaction_like = build_reaction(
        &ctx.subscriber.keys,
        SEED_TOPIC_ALPHA,
        &post_alpha,
        7,
        "👍",
        base + 1800,
    )?;
    let reaction_repost = build_reaction(
        &ctx.subscriber.keys,
        SEED_TOPIC_ALPHA,
        &post_alpha_follow,
        6,
        "",
        base + 2100,
    )?;

    Ok(vec![
        SeedEvent {
            raw: post_alpha,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
        SeedEvent {
            raw: post_alpha_extra_one,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
        SeedEvent {
            raw: post_alpha_extra_two,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
        SeedEvent {
            raw: post_alpha_extra_three,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
        SeedEvent {
            raw: post_alpha_extra_four,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
        SeedEvent {
            raw: post_alpha_follow,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
        SeedEvent {
            raw: post_beta,
            topic_id: SEED_TOPIC_BETA.to_string(),
        },
        SeedEvent {
            raw: reaction_like,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
        SeedEvent {
            raw: reaction_repost,
            topic_id: SEED_TOPIC_ALPHA.to_string(),
        },
    ])
}

fn build_post(
    keys: &Keys,
    topic_id: &str,
    title: &str,
    content: &str,
    created_at: i64,
) -> Result<nostr::RawEvent> {
    let tags = vec![
        vec!["t".to_string(), topic_id.to_string()],
        vec!["title".to_string(), title.to_string()],
        seed_tag(),
        vec!["k".to_string(), "kukuri".to_string()],
        vec!["ver".to_string(), "1".to_string()],
    ];
    build_event_at(keys, 1, tags, content.to_string(), created_at)
}

fn build_reaction(
    keys: &Keys,
    topic_id: &str,
    target: &nostr::RawEvent,
    kind: u16,
    content: &str,
    created_at: i64,
) -> Result<nostr::RawEvent> {
    let tags = vec![
        vec!["e".to_string(), target.id.clone()],
        vec!["p".to_string(), target.pubkey.clone()],
        vec!["t".to_string(), topic_id.to_string()],
        seed_tag(),
        vec!["k".to_string(), "kukuri".to_string()],
        vec!["ver".to_string(), "1".to_string()],
    ];
    build_event_at(keys, kind, tags, content.to_string(), created_at)
}

fn build_event_at(
    keys: &Keys,
    kind: u16,
    tags: Vec<Vec<String>>,
    content: String,
    created_at: i64,
) -> Result<nostr::RawEvent> {
    let mut builder = EventBuilder::new(Kind::Custom(kind), content)
        .custom_created_at(Timestamp::from_secs(created_at.max(0) as u64));
    for tag in tags {
        if tag.is_empty() {
            continue;
        }
        let kind = TagKind::from(tag[0].as_str());
        let values = if tag.len() > 1 { tag[1..].to_vec() } else { Vec::new() };
        builder = builder.tag(Tag::custom(kind, values));
    }
    let signed = builder.sign_with_keys(keys)?;
    let value = serde_json::to_value(&signed)?;
    nostr::parse_event(&value)
}

fn seed_tag() -> Vec<String> {
    vec![SEED_TAG_KEY.to_string(), SEED_TAG_VALUE.to_string()]
}

async fn upsert_subscriber(pool: &Pool<Postgres>, pubkey: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_user.subscriber_accounts          (subscriber_pubkey, status)          VALUES ($1, 'active')          ON CONFLICT (subscriber_pubkey) DO UPDATE SET status = 'active', updated_at = NOW()",
    )
    .bind(pubkey)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_topic_subscription(
    pool: &Pool<Postgres>,
    topic_id: &str,
    pubkey: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_user.topic_subscriptions          (topic_id, subscriber_pubkey, status)          VALUES ($1, $2, 'active')          ON CONFLICT (topic_id, subscriber_pubkey) DO UPDATE SET status = 'active', ended_at = NULL",
    )
    .bind(topic_id)
    .bind(pubkey)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_node_subscription(pool: &Pool<Postgres>, topic_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_admin.node_subscriptions          (topic_id, enabled, ref_count)          VALUES ($1, TRUE, 1)          ON CONFLICT (topic_id) DO UPDATE SET enabled = TRUE, ref_count = GREATEST(cn_admin.node_subscriptions.ref_count, 1), updated_at = NOW()",
    )
    .bind(topic_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_topic_service(
    pool: &Pool<Postgres>,
    topic_id: &str,
    role: &str,
    scope: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_admin.topic_services          (topic_id, role, scope, is_active, updated_by)          VALUES ($1, $2, $3, TRUE, $4)          ON CONFLICT (topic_id, role, scope) DO UPDATE SET is_active = TRUE, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
    )
    .bind(topic_id)
    .bind(role)
    .bind(scope)
    .bind(SEED_SOURCE)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_relay_event(pool: &Pool<Postgres>, event: &nostr::RawEvent) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_relay.events          (event_id, pubkey, kind, created_at, tags, content, sig, raw_json, ingested_at, is_deleted, is_ephemeral, is_current, replaceable_key, addressable_key, expires_at)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), FALSE, FALSE, TRUE, NULL, NULL, NULL)          ON CONFLICT (event_id) DO NOTHING",
    )
    .bind(&event.id)
    .bind(&event.pubkey)
    .bind(event.kind as i32)
    .bind(event.created_at)
    .bind(serde_json::to_value(&event.tags)?)
    .bind(&event.content)
    .bind(&event.sig)
    .bind(serde_json::to_value(event)?)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_event_topic(pool: &Pool<Postgres>, event_id: &str, topic_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_relay.event_topics (event_id, topic_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(event_id)
    .bind(topic_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_meili_documents(
    meili: &meili::MeiliClient,
    events: &[SeedEvent],
) -> Result<()> {
    let mut docs_by_topic: BTreeMap<String, Vec<IndexDocument>> = BTreeMap::new();
    for event in events {
        if event.raw.kind != 1 {
            continue;
        }
        let doc = build_document(&event.raw, &event.topic_id);
        docs_by_topic
            .entry(event.topic_id.clone())
            .or_default()
            .push(doc);
    }

    for (topic_id, docs) in docs_by_topic {
        let uid = meili::topic_index_uid(&topic_id);
        meili
            .ensure_index(&uid, "event_id", Some(default_index_settings()))
            .await?;
        meili.upsert_documents(&uid, &docs).await?;
    }
    Ok(())
}

async fn insert_label(
    pool: &Pool<Postgres>,
    label_event: &nostr::RawEvent,
    input: &moderation::LabelInput,
    now: i64,
) -> Result<()> {
    let issued_at = chrono::Utc
        .timestamp_opt(now, 0)
        .single()
        .unwrap_or_else(|| chrono::Utc::now());
    sqlx::query(
        "INSERT INTO cn_moderation.labels          (label_id, source_event_id, target, topic_id, label, confidence, policy_url, policy_ref, exp, issuer_pubkey, rule_id, source, label_event_json, issued_at)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, $11, $12, $13)          ON CONFLICT (label_id) DO NOTHING",
    )
    .bind(&label_event.id)
    .bind::<Option<&str>>(None)
    .bind(&input.target)
    .bind(&input.topic_id)
    .bind(&input.label)
    .bind(input.confidence)
    .bind(&input.policy_url)
    .bind(&input.policy_ref)
    .bind(input.exp)
    .bind(&label_event.pubkey)
    .bind(SEED_SOURCE)
    .bind(serde_json::to_value(label_event)?)
    .bind(issued_at)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_attestation(
    pool: &Pool<Postgres>,
    event: &nostr::RawEvent,
    subject: &str,
    claim: &str,
    score: f64,
    exp: i64,
    value: &Value,
    context: &Value,
    topic_id: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO cn_trust.attestations          (attestation_id, subject, claim, score, exp, topic_id, issuer_pubkey, value_json, evidence_json, context_json, event_json)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)          ON CONFLICT (attestation_id) DO NOTHING",
    )
    .bind(&event.id)
    .bind(subject)
    .bind(claim)
    .bind(score)
    .bind(exp)
    .bind(topic_id)
    .bind(&event.pubkey)
    .bind(value)
    .bind(json!([]))
    .bind(context)
    .bind(serde_json::to_value(event)?)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_report_score(
    pool: &Pool<Postgres>,
    subject_pubkey: &str,
    attestation_id: String,
    attestation_exp: i64,
) -> Result<()> {
    let now = auth::unix_seconds()? as i64;
    let since = now.saturating_sub(7 * 86400);
    sqlx::query(
        "INSERT INTO cn_trust.report_scores          (subject_pubkey, score, report_count, label_count, window_start, window_end, attestation_id, attestation_exp)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)          ON CONFLICT (subject_pubkey) DO UPDATE SET score = EXCLUDED.score,              report_count = EXCLUDED.report_count,              label_count = EXCLUDED.label_count,              window_start = EXCLUDED.window_start,              window_end = EXCLUDED.window_end,              attestation_id = EXCLUDED.attestation_id,              attestation_exp = EXCLUDED.attestation_exp,              updated_at = NOW()",
    )
    .bind(subject_pubkey)
    .bind(0.35)
    .bind(2_i64)
    .bind(1_i64)
    .bind(since)
    .bind(now)
    .bind(attestation_id)
    .bind(attestation_exp)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_communication_score(
    pool: &Pool<Postgres>,
    subject_pubkey: &str,
    attestation_id: String,
    attestation_exp: i64,
) -> Result<()> {
    let now = auth::unix_seconds()? as i64;
    let since = now.saturating_sub(7 * 86400);
    sqlx::query(
        "INSERT INTO cn_trust.communication_scores          (subject_pubkey, score, interaction_count, peer_count, window_start, window_end, attestation_id, attestation_exp)          VALUES ($1, $2, $3, $4, $5, $6, $7, $8)          ON CONFLICT (subject_pubkey) DO UPDATE SET score = EXCLUDED.score,              interaction_count = EXCLUDED.interaction_count,              peer_count = EXCLUDED.peer_count,              window_start = EXCLUDED.window_start,              window_end = EXCLUDED.window_end,              attestation_id = EXCLUDED.attestation_id,              attestation_exp = EXCLUDED.attestation_exp,              updated_at = NOW()",
    )
    .bind(subject_pubkey)
    .bind(0.78)
    .bind(4_i64)
    .bind(2_i64)
    .bind(since)
    .bind(now)
    .bind(attestation_id)
    .bind(attestation_exp)
    .execute(pool)
    .await?;
    Ok(())
}

fn build_document(raw: &nostr::RawEvent, topic_id: &str) -> IndexDocument {
    IndexDocument {
        event_id: raw.id.clone(),
        topic_id: topic_id.to_string(),
        kind: raw.kind as i32,
        author: raw.pubkey.clone(),
        created_at: raw.created_at,
        title: normalize_title(raw),
        summary: normalize_summary(&raw.content),
        content: raw.content.clone(),
        tags: normalize_tags(raw),
    }
}

fn normalize_title(raw: &nostr::RawEvent) -> String {
    let from_tag = raw
        .first_tag_value("title")
        .or_else(|| raw.first_tag_value("subject"))
        .unwrap_or_default();
    if !from_tag.trim().is_empty() {
        return truncate_chars(from_tag.trim(), 80);
    }
    let first_line = raw.content.lines().next().unwrap_or("").trim();
    truncate_chars(first_line, 80)
}

fn normalize_summary(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    truncate_chars(trimmed, 200)
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    value.chars().take(max).collect()
}

fn normalize_tags(raw: &nostr::RawEvent) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut tags = Vec::new();
    for tag in raw.tag_values("t") {
        if seen.insert(tag.clone()) {
            tags.push(tag);
        }
    }
    tags
}

fn default_index_settings() -> Value {
    json!({
        "searchableAttributes": ["title", "summary", "content", "author", "tags"],
        "filterableAttributes": ["author", "kind", "created_at", "tags"],
        "sortableAttributes": ["created_at"]
    })
}
