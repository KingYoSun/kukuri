use base64::prelude::*;
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

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
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

async fn cleanup_admin_user_and_audits(pool: &Pool<Postgres>, username: &str) {
    let target = format!("admin_user:{username}");
    sqlx::query("DELETE FROM cn_admin.audit_logs WHERE target = $1")
        .bind(&target)
        .execute(pool)
        .await
        .expect("cleanup admin audit logs");
    sqlx::query(
        "DELETE FROM cn_admin.admin_sessions          WHERE admin_user_id IN (SELECT admin_user_id FROM cn_admin.admin_users WHERE username = $1)",
    )
    .bind(username)
    .execute(pool)
    .await
    .expect("cleanup admin sessions");
    sqlx::query("DELETE FROM cn_admin.admin_users WHERE username = $1")
        .bind(username)
        .execute(pool)
        .await
        .expect("cleanup admin user");
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

#[tokio::test]
async fn cli_smoke_covers_migrate_config_admin_openapi_and_p2p_commands() {
    let _guard = acquire_integration_test_lock().await;
    let pool = connect_pool().await;
    ensure_migrated(&pool).await;
    let db_url = database_url();

    let migrate_output = run_cn(&["migrate"], &db_url, None);
    assert_success(&migrate_output, "migrate");
    let audit_table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('cn_admin.audit_logs')::text")
            .fetch_one(&pool)
            .await
            .expect("load audit table regclass");
    assert_eq!(audit_table.as_deref(), Some("cn_admin.audit_logs"));

    let config_seed_output = run_cn(&["config", "seed"], &db_url, None);
    assert_success(&config_seed_output, "config seed");
    let service_config_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cn_admin.service_configs")
            .fetch_one(&pool)
            .await
            .expect("count service configs");
    assert!(
        service_config_count > 0,
        "config seed should ensure cn_admin.service_configs is populated"
    );

    let admin_count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cn_admin.admin_users")
        .fetch_one(&pool)
        .await
        .expect("count admin users before bootstrap");
    let bootstrap_username = format!("cli_smoke_bootstrap_{}", unique_suffix());
    let bootstrap_output = run_cn(
        &[
            "admin",
            "bootstrap",
            "--username",
            bootstrap_username.as_str(),
            "--password",
            "bootstrap-smoke-pass",
        ],
        &db_url,
        None,
    );
    assert_success(&bootstrap_output, "admin bootstrap");
    let admin_count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM cn_admin.admin_users")
        .fetch_one(&pool)
        .await
        .expect("count admin users after bootstrap");
    let bootstrap_user_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cn_admin.admin_users WHERE username = $1)")
            .bind(&bootstrap_username)
            .fetch_one(&pool)
            .await
            .expect("check bootstrap user existence");
    assert!(
        admin_count_after >= admin_count_before,
        "admin count should not decrease"
    );
    if bootstrap_user_exists {
        assert_eq!(
            admin_count_after,
            admin_count_before + 1,
            "bootstrap should add one admin when new user is inserted"
        );
    } else {
        assert_eq!(
            admin_count_after, admin_count_before,
            "bootstrap should be no-op when an admin already exists"
        );
    }

    let reset_username = format!("cli_smoke_reset_{}", unique_suffix());
    sqlx::query(
        "INSERT INTO cn_admin.admin_users          (admin_user_id, username, password_hash, is_active)          VALUES ($1, $2, $3, TRUE)",
    )
    .bind(format!("admin-{}", unique_suffix()))
    .bind(&reset_username)
    .bind("before-reset")
    .execute(&pool)
    .await
    .expect("insert reset-password fixture user");
    let reset_output = run_cn(
        &[
            "admin",
            "reset-password",
            "--username",
            reset_username.as_str(),
            "--password",
            "after-reset",
        ],
        &db_url,
        None,
    );
    assert_success(&reset_output, "admin reset-password");
    let password_hash_after: String =
        sqlx::query_scalar("SELECT password_hash FROM cn_admin.admin_users WHERE username = $1")
            .bind(&reset_username)
            .fetch_one(&pool)
            .await
            .expect("load password hash after reset");
    assert_ne!(password_hash_after, "before-reset");

    let openapi_user_path = temp_path("openapi_user_api.json");
    let openapi_user_path_arg = openapi_user_path.to_string_lossy().to_string();
    let openapi_user_output = run_cn(
        &[
            "openapi",
            "export",
            "--service",
            "user-api",
            "--output",
            openapi_user_path_arg.as_str(),
            "--pretty",
        ],
        &db_url,
        None,
    );
    assert_success(&openapi_user_output, "openapi export --service user-api");
    let openapi_user_doc: Value = serde_json::from_str(
        &fs::read_to_string(&openapi_user_path).expect("read user-api openapi output"),
    )
    .expect("parse user-api openapi output");
    assert!(
        openapi_user_doc
            .get("openapi")
            .and_then(Value::as_str)
            .is_some(),
        "OpenAPI document must include openapi version"
    );
    assert!(
        openapi_user_doc
            .get("paths")
            .and_then(Value::as_object)
            .is_some_and(|paths| !paths.is_empty()),
        "OpenAPI document must include non-empty paths"
    );

    let openapi_admin_path = temp_path("openapi_admin_api.json");
    let openapi_admin_path_arg = openapi_admin_path.to_string_lossy().to_string();
    let openapi_admin_output = run_cn(
        &[
            "openapi",
            "export",
            "--service",
            "admin-api",
            "--output",
            openapi_admin_path_arg.as_str(),
        ],
        &db_url,
        None,
    );
    assert_success(&openapi_admin_output, "openapi export --service admin-api");

    let p2p_help_output = run_cn(&["p2p", "--help"], &db_url, None);
    assert_success(&p2p_help_output, "p2p --help");
    let p2p_help = stdout_text(&p2p_help_output);
    for expected in ["node-id", "bootstrap", "relay", "connect"] {
        assert!(
            p2p_help.contains(expected),
            "p2p help output must include `{expected}` subcommand"
        );
    }

    let secret_key = BASE64_STANDARD.encode([7u8; 32]);
    let p2p_node_id_output_1 = run_cn(
        &[
            "p2p",
            "node-id",
            "--bind",
            "127.0.0.1:0",
            "--log-level",
            "error",
            "--secret-key",
            secret_key.as_str(),
        ],
        &db_url,
        None,
    );
    assert_success(&p2p_node_id_output_1, "p2p node-id first run");
    let node_id_1 = stdout_trimmed(&p2p_node_id_output_1);
    assert_hex_pubkey(&node_id_1);

    let p2p_node_id_output_2 = run_cn(
        &[
            "p2p",
            "node-id",
            "--bind",
            "127.0.0.1:0",
            "--log-level",
            "error",
            "--secret-key",
            secret_key.as_str(),
        ],
        &db_url,
        None,
    );
    assert_success(&p2p_node_id_output_2, "p2p node-id second run");
    let node_id_2 = stdout_trimmed(&p2p_node_id_output_2);
    assert_eq!(
        node_id_1, node_id_2,
        "p2p node-id should stay deterministic for the same secret key"
    );

    let bootstrap_help_output = run_cn(&["bootstrap", "--help"], &db_url, None);
    assert_success(&bootstrap_help_output, "bootstrap --help");
    assert!(
        stdout_text(&bootstrap_help_output).contains("Usage: cn bootstrap"),
        "bootstrap help output should document top-level `cn bootstrap`"
    );

    let relay_help_output = run_cn(&["relay", "--help"], &db_url, None);
    assert_success(&relay_help_output, "relay --help");
    assert!(
        stdout_text(&relay_help_output).contains("Usage: cn relay"),
        "relay help output should document top-level `cn relay`"
    );

    cleanup_admin_user_and_audits(&pool, &reset_username).await;
    cleanup_admin_user_and_audits(&pool, &bootstrap_username).await;
    let _ = fs::remove_file(&openapi_user_path);
    let _ = fs::remove_file(&openapi_admin_path);
}
