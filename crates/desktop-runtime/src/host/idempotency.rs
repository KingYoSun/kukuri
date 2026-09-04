use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use uuid::Uuid;

pub const IDEMPOTENCY_LEDGER_FILE_NAME: &str = "kukuri.idempotency.sqlite3";
const RETENTION_MS: i64 = 30 * 24 * 60 * 60 * 1000;
const MAX_SCOPE_RECORDS: i64 = 10_000;
const MAX_CLOCK_SKEW_MS: i64 = 5 * 60 * 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdempotencyScope<'a> {
    pub profile: &'a str,
    pub account: &'a str,
    pub command: &'a str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IdempotencyClaim {
    Execute,
    Replay(Value),
    Conflict,
    OutcomeUnknown,
    Expired,
}

pub struct IdempotencyLedger {
    pool: SqlitePool,
    digest_key: [u8; 32],
}

impl IdempotencyLedger {
    pub async fn open(db_path: &Path) -> Result<Self> {
        let path = idempotency_ledger_path(db_path);
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Delete)
            .synchronous(SqliteSynchronous::Full);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .with_context(|| format!("failed to open idempotency ledger `{}`", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .context("failed to protect idempotency ledger")?;
        }
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS records (
                profile TEXT NOT NULL,
                account TEXT NOT NULL,
                command TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                payload_hash TEXT NOT NULL,
                state TEXT NOT NULL,
                result_json TEXT,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (profile, account, command, idempotency_key)
            )",
        )
        .execute(&pool)
        .await?;
        let salt = match sqlx::query_scalar::<_, String>(
            "SELECT value FROM metadata WHERE key = 'digest_salt'",
        )
        .fetch_optional(&pool)
        .await?
        {
            Some(value) => value,
            None => {
                let value = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
                sqlx::query("INSERT INTO metadata (key, value) VALUES ('digest_salt', ?)")
                    .bind(&value)
                    .execute(&pool)
                    .await?;
                value
            }
        };
        let digest_key = *blake3::hash(salt.as_bytes()).as_bytes();
        Ok(Self { pool, digest_key })
    }

    pub fn digest_payload(&self, canonical_payload: &[u8], secret: Option<&[u8]>) -> String {
        let mut hasher = blake3::Hasher::new_keyed(&self.digest_key);
        hasher.update(&(canonical_payload.len() as u64).to_be_bytes());
        hasher.update(canonical_payload);
        if let Some(secret) = secret {
            hasher.update(&(secret.len() as u64).to_be_bytes());
            hasher.update(secret);
        }
        hasher.finalize().to_hex().to_string()
    }

    pub async fn claim(
        &self,
        scope: &IdempotencyScope<'_>,
        key: &str,
        payload_hash: &str,
        now_ms: i64,
    ) -> Result<IdempotencyClaim> {
        let key_ms = uuid_v7_millis(key)?;
        if key_ms > now_ms.saturating_add(MAX_CLOCK_SKEW_MS) {
            bail!("idempotency_key timestamp is too far in the future");
        }
        self.prune(scope, now_ms).await?;
        let mut transaction = self.pool.begin().await?;
        if let Some(row) = sqlx::query(
            "SELECT payload_hash, state, result_json FROM records
             WHERE profile = ? AND account = ? AND command = ? AND idempotency_key = ?",
        )
        .bind(scope.profile)
        .bind(scope.account)
        .bind(scope.command)
        .bind(key)
        .fetch_optional(&mut *transaction)
        .await?
        {
            let existing_hash: String = row.try_get("payload_hash")?;
            if existing_hash != payload_hash {
                transaction.commit().await?;
                return Ok(IdempotencyClaim::Conflict);
            }
            let state: String = row.try_get("state")?;
            let claim = match state.as_str() {
                "completed" => {
                    let result: Option<String> = row.try_get("result_json")?;
                    let value = serde_json::from_str(
                        result
                            .as_deref()
                            .context("completed idempotency record has no result")?,
                    )
                    .context("completed idempotency result is invalid")?;
                    IdempotencyClaim::Replay(value)
                }
                "in_progress" | "unknown" => IdempotencyClaim::OutcomeUnknown,
                _ => bail!("idempotency ledger contains an invalid state"),
            };
            transaction.commit().await?;
            return Ok(claim);
        }

        let restore_ms = metadata_i64(&mut transaction, "restored_at_ms").await?;
        if restore_ms.is_some_and(|value| key_ms <= value.saturating_add(MAX_CLOCK_SKEW_MS)) {
            transaction.commit().await?;
            return Ok(IdempotencyClaim::OutcomeUnknown);
        }
        if key_ms < now_ms.saturating_sub(RETENTION_MS) {
            transaction.commit().await?;
            return Ok(IdempotencyClaim::Expired);
        }
        let watermark_ms = metadata_i64(&mut transaction, "retention_watermark_ms").await?;
        if watermark_ms.is_some_and(|value| key_ms <= value) {
            transaction.commit().await?;
            return Ok(IdempotencyClaim::Expired);
        }
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM records WHERE profile = ? AND account = ?")
                .bind(scope.profile)
                .bind(scope.account)
                .fetch_one(&mut *transaction)
                .await?;
        if count >= MAX_SCOPE_RECORDS {
            bail!("idempotency ledger capacity is exhausted by unresolved records");
        }
        sqlx::query(
            "INSERT INTO records (
                profile, account, command, idempotency_key, payload_hash, state,
                result_json, created_at_ms, updated_at_ms
             ) VALUES (?, ?, ?, ?, ?, 'in_progress', NULL, ?, ?)",
        )
        .bind(scope.profile)
        .bind(scope.account)
        .bind(scope.command)
        .bind(key)
        .bind(payload_hash)
        .bind(now_ms)
        .bind(now_ms)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(IdempotencyClaim::Execute)
    }

    pub async fn complete(
        &self,
        scope: &IdempotencyScope<'_>,
        key: &str,
        result: &Value,
        now_ms: i64,
    ) -> Result<()> {
        let result =
            serde_json::to_string(result).context("failed to encode idempotency result")?;
        let updated = sqlx::query(
            "UPDATE records SET state = 'completed', result_json = ?, updated_at_ms = ?
             WHERE profile = ? AND account = ? AND command = ? AND idempotency_key = ?
               AND state = 'in_progress'",
        )
        .bind(result)
        .bind(now_ms)
        .bind(scope.profile)
        .bind(scope.account)
        .bind(scope.command)
        .bind(key)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if updated != 1 {
            bail!("idempotency record was not in progress");
        }
        Ok(())
    }

    pub async fn mark_unknown(
        &self,
        scope: &IdempotencyScope<'_>,
        key: &str,
        now_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE records SET state = 'unknown', result_json = NULL, updated_at_ms = ?
             WHERE profile = ? AND account = ? AND command = ? AND idempotency_key = ?
               AND state = 'in_progress'",
        )
        .bind(now_ms)
        .bind(scope.profile)
        .bind(scope.account)
        .bind(scope.command)
        .bind(key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_restored(&self, now_ms: i64) -> Result<()> {
        set_metadata_i64(&self.pool, "restored_at_ms", now_ms).await
    }

    pub async fn close(self) {
        self.pool.close().await;
    }

    async fn prune(&self, scope: &IdempotencyScope<'_>, now_ms: i64) -> Result<()> {
        let cutoff = now_ms.saturating_sub(RETENTION_MS);
        let mut transaction = self.pool.begin().await?;
        let removed_time: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(updated_at_ms) FROM records
             WHERE profile = ? AND account = ? AND state = 'completed' AND updated_at_ms < ?",
        )
        .bind(scope.profile)
        .bind(scope.account)
        .bind(cutoff)
        .fetch_one(&mut *transaction)
        .await?;
        sqlx::query(
            "DELETE FROM records
             WHERE profile = ? AND account = ? AND state = 'completed' AND updated_at_ms < ?",
        )
        .bind(scope.profile)
        .bind(scope.account)
        .bind(cutoff)
        .execute(&mut *transaction)
        .await?;
        if let Some(removed_time) = removed_time {
            let previous = metadata_i64(&mut transaction, "retention_watermark_ms")
                .await?
                .unwrap_or(0);
            set_metadata_i64_tx(
                &mut transaction,
                "retention_watermark_ms",
                previous.max(removed_time),
            )
            .await?;
        }
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM records WHERE profile = ? AND account = ?")
                .bind(scope.profile)
                .bind(scope.account)
                .fetch_one(&mut *transaction)
                .await?;
        let required_removals = count.saturating_sub(MAX_SCOPE_RECORDS - 1);
        if required_removals > 0 {
            let keys = sqlx::query_as::<_, (String, String)>(
                "SELECT command, idempotency_key FROM records
                 WHERE profile = ? AND account = ? AND state = 'completed'
                 ORDER BY updated_at_ms ASC, command ASC, idempotency_key ASC LIMIT ?",
            )
            .bind(scope.profile)
            .bind(scope.account)
            .bind(required_removals)
            .fetch_all(&mut *transaction)
            .await?;
            if keys.len() < required_removals as usize {
                bail!("idempotency ledger capacity is exhausted by unresolved records");
            }
            let mut removed_key_ms = 0;
            for (command, key) in keys {
                removed_key_ms = removed_key_ms.max(uuid_v7_millis(&key)?);
                sqlx::query(
                    "DELETE FROM records
                     WHERE profile = ? AND account = ? AND command = ? AND idempotency_key = ?",
                )
                .bind(scope.profile)
                .bind(scope.account)
                .bind(command)
                .bind(key)
                .execute(&mut *transaction)
                .await?;
            }
            let previous = metadata_i64(&mut transaction, "retention_watermark_ms")
                .await?
                .unwrap_or(0);
            set_metadata_i64_tx(
                &mut transaction,
                "retention_watermark_ms",
                previous.max(removed_key_ms),
            )
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}

pub fn idempotency_ledger_path(db_path: &Path) -> PathBuf {
    db_path.with_file_name(IDEMPOTENCY_LEDGER_FILE_NAME)
}

fn uuid_v7_millis(key: &str) -> Result<i64> {
    let uuid = Uuid::parse_str(key).context("idempotency_key must be a UUID")?;
    if uuid.get_version_num() != 7 {
        bail!("idempotency_key must be UUIDv7");
    }
    let (seconds, nanos) = uuid
        .get_timestamp()
        .context("idempotency_key has no timestamp")?
        .to_unix();
    i64::try_from(seconds.saturating_mul(1000) + u64::from(nanos / 1_000_000))
        .context("idempotency_key timestamp is out of range")
}

async fn metadata_i64(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
) -> Result<Option<i64>> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM metadata WHERE key = ?")
        .bind(key)
        .fetch_optional(&mut **transaction)
        .await?;
    value
        .map(|value| value.parse::<i64>().context("invalid ledger metadata"))
        .transpose()
}

async fn set_metadata_i64(pool: &SqlitePool, key: &str, value: i64) -> Result<()> {
    sqlx::query(
        "INSERT INTO metadata (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

async fn set_metadata_i64_tx(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
    value: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO metadata (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn unix_millis() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }

    #[tokio::test]
    async fn replay_conflict_and_crash_unknown_are_durable() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let key = Uuid::now_v7().to_string();
        let scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.write",
        };
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        assert_eq!(
            ledger
                .claim(&scope, &key, "hash-a", unix_millis())
                .await
                .expect("claim"),
            IdempotencyClaim::Execute
        );
        drop(ledger);
        let ledger = IdempotencyLedger::open(&db_path).await.expect("reopen");
        assert_eq!(
            ledger
                .claim(&scope, &key, "hash-a", unix_millis())
                .await
                .expect("unknown"),
            IdempotencyClaim::OutcomeUnknown
        );
        assert_eq!(
            ledger
                .claim(&scope, &key, "hash-b", unix_millis())
                .await
                .expect("conflict"),
            IdempotencyClaim::Conflict
        );

        let completed_key = Uuid::now_v7().to_string();
        assert_eq!(
            ledger
                .claim(&scope, &completed_key, "hash-c", unix_millis())
                .await
                .expect("claim"),
            IdempotencyClaim::Execute
        );
        ledger
            .complete(
                &scope,
                &completed_key,
                &json!({"id": "post-1"}),
                unix_millis(),
            )
            .await
            .expect("complete");
        assert_eq!(
            ledger
                .claim(&scope, &completed_key, "hash-c", unix_millis())
                .await
                .expect("replay"),
            IdempotencyClaim::Replay(json!({"id": "post-1"}))
        );
    }

    #[tokio::test]
    async fn secret_bytes_are_keyed_and_never_persisted() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        let sentinel = b"do-not-persist-this-secret";
        let digest = ledger.digest_payload(b"{}", Some(sentinel));
        assert!(!digest.contains(std::str::from_utf8(sentinel).expect("utf8")));
        drop(ledger);
        let bytes = std::fs::read(idempotency_ledger_path(&db_path)).expect("ledger bytes");
        assert!(
            !bytes
                .windows(sentinel.len())
                .any(|window| window == sentinel)
        );
    }

    #[tokio::test]
    async fn missing_key_from_before_restore_is_never_executed() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let key = Uuid::now_v7().to_string();
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        ledger
            .mark_restored(unix_millis().saturating_add(1))
            .await
            .expect("restore marker");
        let scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.write",
        };
        assert_eq!(
            ledger
                .claim(&scope, &key, "hash", unix_millis().saturating_add(2))
                .await
                .expect("claim"),
            IdempotencyClaim::OutcomeUnknown
        );
    }

    #[tokio::test]
    async fn missing_key_older_than_retention_is_expired() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let now_ms = unix_millis();
        let old_ms = now_ms - RETENTION_MS - 1;
        let timestamp = uuid::Timestamp::from_unix(
            uuid::NoContext,
            (old_ms / 1000) as u64,
            ((old_ms % 1000) * 1_000_000) as u32,
        );
        let key = Uuid::new_v7(timestamp).to_string();
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        let scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.write",
        };
        assert_eq!(
            ledger
                .claim(&scope, &key, "hash", now_ms)
                .await
                .expect("claim"),
            IdempotencyClaim::Expired
        );
    }

    #[tokio::test]
    async fn future_key_cannot_bypass_restore_marker() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let now_ms = unix_millis();
        let future_ms = now_ms + MAX_CLOCK_SKEW_MS + 1;
        let timestamp = uuid::Timestamp::from_unix(
            uuid::NoContext,
            (future_ms / 1000) as u64,
            ((future_ms % 1000) * 1_000_000) as u32,
        );
        let key = Uuid::new_v7(timestamp).to_string();
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        ledger.mark_restored(now_ms).await.expect("restore marker");
        let scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.write",
        };
        let error = ledger
            .claim(&scope, &key, "hash", now_ms)
            .await
            .expect_err("future key");
        assert!(error.to_string().contains("future"));
    }

    #[tokio::test]
    async fn concurrent_same_key_has_one_executor() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        let scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.write",
        };
        let key = Uuid::now_v7().to_string();
        let now_ms = unix_millis();
        let (left, right) = tokio::join!(
            ledger.claim(&scope, &key, "hash", now_ms),
            ledger.claim(&scope, &key, "hash", now_ms)
        );
        let claims = [left.expect("left claim"), right.expect("right claim")];
        assert_eq!(
            claims
                .iter()
                .filter(|claim| **claim == IdempotencyClaim::Execute)
                .count(),
            1
        );
        assert_eq!(
            claims
                .iter()
                .filter(|claim| **claim == IdempotencyClaim::OutcomeUnknown)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn unresolved_capacity_limit_fails_closed() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        let now_ms = unix_millis();
        let mut transaction = ledger.pool.begin().await.expect("transaction");
        for index in 0..MAX_SCOPE_RECORDS {
            sqlx::query(
                "INSERT INTO records (
                    profile, account, command, idempotency_key, payload_hash, state,
                    result_json, created_at_ms, updated_at_ms
                 ) VALUES ('test', 'account-a', 'fixture.write', ?, 'hash', 'unknown', NULL, ?, ?)",
            )
            .bind(format!("fixture-{index}"))
            .bind(now_ms)
            .bind(now_ms)
            .execute(&mut *transaction)
            .await
            .expect("fixture record");
        }
        transaction.commit().await.expect("commit fixtures");
        let scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.write",
        };
        let error = ledger
            .claim(&scope, &Uuid::now_v7().to_string(), "hash", now_ms)
            .await
            .expect_err("capacity must fail closed");
        assert!(error.to_string().contains("capacity is exhausted"));
    }

    #[tokio::test]
    async fn completed_records_are_trimmed_at_capacity() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        let ledger = IdempotencyLedger::open(&db_path).await.expect("ledger");
        let now_ms = unix_millis();
        let mut oldest_key = String::new();
        let mut transaction = ledger.pool.begin().await.expect("transaction");
        for index in 0..(MAX_SCOPE_RECORDS - 1) {
            let key_ms = now_ms - MAX_SCOPE_RECORDS + index;
            let timestamp = uuid::Timestamp::from_unix(
                uuid::NoContext,
                (key_ms / 1000) as u64,
                ((key_ms % 1000) * 1_000_000) as u32,
            );
            let key = Uuid::new_v7(timestamp).to_string();
            if index == 0 {
                oldest_key.clone_from(&key);
            }
            sqlx::query(
                "INSERT INTO records (
                    profile, account, command, idempotency_key, payload_hash, state,
                    result_json, created_at_ms, updated_at_ms
                 ) VALUES ('test', 'account-a', 'fixture.write', ?, 'hash', 'completed', '{}', ?, ?)",
            )
            .bind(key)
            .bind(key_ms)
            .bind(key_ms)
            .execute(&mut *transaction)
            .await
            .expect("fixture record");
        }
        sqlx::query(
            "INSERT INTO records (
                profile, account, command, idempotency_key, payload_hash, state,
                result_json, created_at_ms, updated_at_ms
             ) VALUES ('test', 'account-a', 'fixture.other-write', ?, 'hash', 'completed', '{}', ?, ?)",
        )
        .bind(&oldest_key)
        .bind(now_ms)
        .bind(now_ms)
        .execute(&mut *transaction)
        .await
        .expect("same-key other-command fixture");
        transaction.commit().await.expect("commit fixtures");
        let scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.write",
        };
        let new_key = Uuid::now_v7().to_string();
        assert_eq!(
            ledger
                .claim(&scope, &new_key, "hash", now_ms)
                .await
                .expect("new claim"),
            IdempotencyClaim::Execute
        );
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM records WHERE profile = 'test' AND account = 'account-a'",
        )
        .fetch_one(&ledger.pool)
        .await
        .expect("record count");
        assert_eq!(count, MAX_SCOPE_RECORDS);
        let other_scope = IdempotencyScope {
            profile: "test",
            account: "account-a",
            command: "fixture.other-write",
        };
        assert_eq!(
            ledger
                .claim(&other_scope, &oldest_key, "hash", now_ms)
                .await
                .expect("same-key other-command replay"),
            IdempotencyClaim::Replay(json!({}))
        );
        assert_eq!(
            ledger
                .claim(&scope, &oldest_key, "hash", now_ms)
                .await
                .expect("trimmed claim"),
            IdempotencyClaim::Expired
        );
    }

    #[tokio::test]
    async fn corrupted_ledger_is_rejected() {
        let root = tempfile::tempdir().expect("tempdir");
        let db_path = root.path().join("kukuri.db");
        std::fs::write(idempotency_ledger_path(&db_path), b"not sqlite")
            .expect("corrupt ledger fixture");
        assert!(IdempotencyLedger::open(&db_path).await.is_err());
    }
}
