use nostr_sdk::prelude::Keys;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, OnceCell};

static MIGRATIONS: OnceCell<()> = OnceCell::const_new();
static INTEGRATION_TEST_LOCK: OnceCell<Arc<Mutex<()>>> = OnceCell::const_new();

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:15432/cn".to_string())
}

async fn connect_pool() -> Pool<Postgres> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url())
        .await
        .expect("connect postgres")
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

async fn acquire_integration_test_lock() -> tokio::sync::OwnedMutexGuard<()> {
    let lock = INTEGRATION_TEST_LOCK
        .get_or_init(|| async { Arc::new(Mutex::new(())) })
        .await
        .clone();
    lock.lock_owned().await
}

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time moved backwards")
        .as_nanos()
        .to_string()
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("kukuri_cn_cli_it_{}_{}", unique_suffix(), name));
    path
}

fn unique_topic_id(label: &str) -> String {
    let raw = format!("kukuri:tauri:{label}:{}", unique_suffix());
    format!(
        "kukuri:tauri:{}",
        hex::encode(blake3::hash(raw.as_bytes()).as_bytes())
    )
}

fn run_cn(args: &[&str], db_url: &str, node_key_path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cn"));
    command.args(args);
    command.env("DATABASE_URL", db_url);
    command.env("RUST_LOG", "off");
    if let Some(path) = node_key_path {
        command.env("NODE_KEY_PATH", path);
    }
    command.output().expect("run cn command")
}

fn assert_success(output: &Output, context: &str) {
    if output.status.success() {
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    panic!(
        "{context} failed\nstatus={}\nstdout=\n{}\nstderr=\n{}",
        output.status, stdout, stderr
    );
}

fn stdout_trimmed(output: &Output) -> String {
    String::from_utf8(output.stdout.clone())
        .expect("stdout is utf-8")
        .trim()
        .to_string()
}

fn parse_json_stdout(output: &Output) -> Value {
    serde_json::from_str(&stdout_trimmed(output)).expect("stdout is json")
}

fn assert_hex_pubkey(pubkey: &str) {
    assert_eq!(pubkey.len(), 64, "pubkey length");
    assert!(
        pubkey.chars().all(|ch| ch.is_ascii_hexdigit()),
        "pubkey is not hex: {pubkey}"
    );
}

async fn cleanup_access_control_rows(pool: &Pool<Postgres>, topic_id: &str) {
    let target_prefix = format!("access_control:{topic_id}%");
    sqlx::query("DELETE FROM cn_admin.audit_logs WHERE target LIKE $1")
        .bind(&target_prefix)
        .execute(pool)
        .await
        .expect("cleanup access control audits");
    sqlx::query("DELETE FROM cn_user.key_envelopes WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup key envelopes");
    sqlx::query("DELETE FROM cn_user.key_envelope_distribution_results WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup key envelope distribution results");
    sqlx::query("DELETE FROM cn_user.topic_memberships WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup topic memberships");
    sqlx::query("DELETE FROM cn_admin.topic_scope_keys WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup topic scope keys");
    sqlx::query("DELETE FROM cn_admin.topic_scope_state WHERE topic_id = $1")
        .bind(topic_id)
        .execute(pool)
        .await
        .expect("cleanup topic scope state");
}

async fn cleanup_node_key_audits(pool: &Pool<Postgres>, pubkeys: &[&str]) {
    for pubkey in pubkeys {
        sqlx::query(
            "DELETE FROM cn_admin.audit_logs              WHERE target = 'node_key'                AND diff_json->>'public_key' = $1",
        )
        .bind(pubkey)
        .execute(pool)
        .await
        .expect("cleanup node key audit");
    }
}

#[tokio::test]
async fn node_key_generate_rotate_records_audit_and_keeps_stdout_shape() {
    let _guard = acquire_integration_test_lock().await;
    let pool = connect_pool().await;
    ensure_migrated(&pool).await;

    let db_url = database_url();
    let key_path = temp_path("node_key.json");
    let key_path_arg = key_path.to_string_lossy().to_string();

    let generate_output = run_cn(
        &["node-key", "generate", "--path", key_path_arg.as_str()],
        &db_url,
        None,
    );
    assert_success(&generate_output, "node-key generate");
    let generated_pubkey = stdout_trimmed(&generate_output);
    assert_hex_pubkey(&generated_pubkey);
    assert!(key_path.exists(), "node key file is generated");

    let generate_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)          FROM cn_admin.audit_logs          WHERE action = 'node_key.generate'            AND target = 'node_key'            AND diff_json->>'public_key' = $1",
    )
    .bind(&generated_pubkey)
    .fetch_one(&pool)
    .await
    .expect("count node_key.generate audit");
    assert_eq!(generate_audit_count, 1, "node_key.generate audit count");

    let rotate_output = run_cn(
        &["node-key", "rotate", "--path", key_path_arg.as_str()],
        &db_url,
        None,
    );
    assert_success(&rotate_output, "node-key rotate");
    let rotated_pubkey = stdout_trimmed(&rotate_output);
    assert_hex_pubkey(&rotated_pubkey);
    assert_ne!(
        generated_pubkey, rotated_pubkey,
        "rotate must change pubkey"
    );

    let rotate_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)          FROM cn_admin.audit_logs          WHERE action = 'node_key.rotate'            AND target = 'node_key'            AND diff_json->>'public_key' = $1",
    )
    .bind(&rotated_pubkey)
    .fetch_one(&pool)
    .await
    .expect("count node_key.rotate audit");
    assert_eq!(rotate_audit_count, 1, "node_key.rotate audit count");

    let show_output = run_cn(
        &["node-key", "show", "--path", key_path_arg.as_str()],
        &db_url,
        None,
    );
    assert_success(&show_output, "node-key show");
    assert_eq!(stdout_trimmed(&show_output), rotated_pubkey);

    cleanup_node_key_audits(&pool, &[&generated_pubkey, &rotated_pubkey]).await;
    let _ = fs::remove_file(&key_path);
}

#[tokio::test]
async fn access_control_rotate_revoke_updates_db_audit_and_preserves_output_shape() {
    let _guard = acquire_integration_test_lock().await;
    let pool = connect_pool().await;
    ensure_migrated(&pool).await;

    let db_url = database_url();
    let topic_id = unique_topic_id("access-control");
    let scope = "invite";
    let member_pubkey = Keys::generate().public_key().to_hex();
    let node_key_path = temp_path("access_control_node_key.json");

    sqlx::query(
        "INSERT INTO cn_user.topic_memberships          (topic_id, scope, pubkey, status)          VALUES ($1, $2, $3, 'active')          ON CONFLICT (topic_id, scope, pubkey)          DO UPDATE SET status = 'active', revoked_at = NULL, revoked_reason = NULL",
    )
    .bind(&topic_id)
    .bind(scope)
    .bind(&member_pubkey)
    .execute(&pool)
    .await
    .expect("insert membership");

    let rotate_output = run_cn(
        &[
            "access-control",
            "rotate",
            "--topic",
            topic_id.as_str(),
            "--scope",
            scope,
        ],
        &db_url,
        Some(&node_key_path),
    );
    assert_success(&rotate_output, "access-control rotate");
    let rotate_json = parse_json_stdout(&rotate_output);
    assert_eq!(
        rotate_json.get("topic_id").and_then(Value::as_str),
        Some(topic_id.as_str())
    );
    assert_eq!(
        rotate_json.get("scope").and_then(Value::as_str),
        Some(scope)
    );
    assert_eq!(
        rotate_json.get("previous_epoch").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        rotate_json.get("new_epoch").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        rotate_json.get("recipients").and_then(Value::as_u64),
        Some(1)
    );
    assert!(
        rotate_json.get("rotation").is_none(),
        "rotate output should keep top-level fields for backward compatibility"
    );

    let current_epoch_after_rotate: i32 = sqlx::query_scalar(
        "SELECT current_epoch FROM cn_admin.topic_scope_state WHERE topic_id = $1 AND scope = $2",
    )
    .bind(&topic_id)
    .bind(scope)
    .fetch_one(&pool)
    .await
    .expect("topic scope state after rotate");
    assert_eq!(current_epoch_after_rotate, 1);

    let key_count_after_rotate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_admin.topic_scope_keys WHERE topic_id = $1 AND scope = $2",
    )
    .bind(&topic_id)
    .bind(scope)
    .fetch_one(&pool)
    .await
    .expect("topic scope key count after rotate");
    assert_eq!(key_count_after_rotate, 1);

    let envelope_count_after_rotate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)          FROM cn_user.key_envelopes          WHERE topic_id = $1            AND scope = $2            AND epoch = 1            AND recipient_pubkey = $3",
    )
    .bind(&topic_id)
    .bind(scope)
    .bind(&member_pubkey)
    .fetch_one(&pool)
    .await
    .expect("key envelope count after rotate");
    assert_eq!(envelope_count_after_rotate, 1);

    let rotate_audit_target = format!("access_control:{topic_id}:{scope}");
    let rotate_audit_row = sqlx::query(
        "SELECT actor_admin_user_id, diff_json          FROM cn_admin.audit_logs          WHERE action = 'access_control.rotate'            AND target = $1          ORDER BY audit_id DESC          LIMIT 1",
    )
    .bind(&rotate_audit_target)
    .fetch_one(&pool)
    .await
    .expect("load access_control.rotate audit");
    let rotate_actor: String = rotate_audit_row
        .try_get("actor_admin_user_id")
        .expect("actor");
    let rotate_diff: Value = rotate_audit_row.try_get("diff_json").expect("diff_json");
    assert_eq!(rotate_actor, "system");
    assert_eq!(
        rotate_diff.get("previous_epoch").and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        rotate_diff.get("new_epoch").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        rotate_diff.get("recipients").and_then(Value::as_u64),
        Some(1)
    );

    let reason = "integration-test revoke";
    let revoke_output = run_cn(
        &[
            "access-control",
            "revoke",
            "--topic",
            topic_id.as_str(),
            "--scope",
            scope,
            "--pubkey",
            member_pubkey.as_str(),
            "--reason",
            reason,
        ],
        &db_url,
        Some(&node_key_path),
    );
    assert_success(&revoke_output, "access-control revoke");
    let revoke_json = parse_json_stdout(&revoke_output);
    assert_eq!(
        revoke_json.get("topic_id").and_then(Value::as_str),
        Some(topic_id.as_str())
    );
    assert_eq!(
        revoke_json.get("scope").and_then(Value::as_str),
        Some(scope)
    );
    assert_eq!(
        revoke_json.get("revoked_pubkey").and_then(Value::as_str),
        Some(member_pubkey.as_str())
    );
    assert_eq!(
        revoke_json.get("previous_epoch").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        revoke_json.get("new_epoch").and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        revoke_json.get("recipients").and_then(Value::as_u64),
        Some(0)
    );
    assert!(
        revoke_json.get("rotation").is_none(),
        "revoke output should keep top-level fields for backward compatibility"
    );

    let membership_row = sqlx::query(
        "SELECT status, revoked_reason, revoked_at IS NOT NULL AS revoked          FROM cn_user.topic_memberships          WHERE topic_id = $1 AND scope = $2 AND pubkey = $3",
    )
    .bind(&topic_id)
    .bind(scope)
    .bind(&member_pubkey)
    .fetch_one(&pool)
    .await
    .expect("membership row after revoke");
    let membership_status: String = membership_row.try_get("status").expect("status");
    let membership_revoked_reason: Option<String> = membership_row
        .try_get("revoked_reason")
        .expect("revoked_reason");
    let membership_revoked: bool = membership_row.try_get("revoked").expect("revoked");
    assert_eq!(membership_status, "revoked");
    assert_eq!(membership_revoked_reason.as_deref(), Some(reason));
    assert!(membership_revoked);

    let current_epoch_after_revoke: i32 = sqlx::query_scalar(
        "SELECT current_epoch FROM cn_admin.topic_scope_state WHERE topic_id = $1 AND scope = $2",
    )
    .bind(&topic_id)
    .bind(scope)
    .fetch_one(&pool)
    .await
    .expect("topic scope state after revoke");
    assert_eq!(current_epoch_after_revoke, 2);

    let key_count_after_revoke: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM cn_admin.topic_scope_keys WHERE topic_id = $1 AND scope = $2",
    )
    .bind(&topic_id)
    .bind(scope)
    .fetch_one(&pool)
    .await
    .expect("topic scope key count after revoke");
    assert_eq!(key_count_after_revoke, 2);

    let envelope_count_epoch2: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)          FROM cn_user.key_envelopes          WHERE topic_id = $1            AND scope = $2            AND epoch = 2",
    )
    .bind(&topic_id)
    .bind(scope)
    .fetch_one(&pool)
    .await
    .expect("epoch2 envelope count");
    assert_eq!(envelope_count_epoch2, 0);

    let revoke_audit_target = format!("access_control:{topic_id}:{scope}:{member_pubkey}");
    let revoke_audit_row = sqlx::query(
        "SELECT actor_admin_user_id, diff_json          FROM cn_admin.audit_logs          WHERE action = 'access_control.revoke'            AND target = $1          ORDER BY audit_id DESC          LIMIT 1",
    )
    .bind(&revoke_audit_target)
    .fetch_one(&pool)
    .await
    .expect("load access_control.revoke audit");
    let revoke_actor: String = revoke_audit_row
        .try_get("actor_admin_user_id")
        .expect("actor");
    let revoke_diff: Value = revoke_audit_row.try_get("diff_json").expect("diff_json");
    assert_eq!(revoke_actor, "system");
    assert_eq!(
        revoke_diff.get("reason").and_then(Value::as_str),
        Some(reason)
    );
    assert_eq!(
        revoke_diff.get("previous_epoch").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        revoke_diff.get("new_epoch").and_then(Value::as_i64),
        Some(2)
    );
    assert_eq!(
        revoke_diff.get("recipients").and_then(Value::as_u64),
        Some(0)
    );

    cleanup_access_control_rows(&pool, &topic_id).await;
    let _ = fs::remove_file(&node_key_path);
}
