use anyhow::Result;
use sqlx::{Pool, Postgres, Row};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

pub const FLAG_SEARCH_READ_BACKEND: &str = "search_read_backend";
pub const FLAG_SEARCH_WRITE_MODE: &str = "search_write_mode";
pub const FLAG_SUGGEST_READ_BACKEND: &str = "suggest_read_backend";
pub const FLAG_SHADOW_SAMPLE_RATE: &str = "shadow_sample_rate";

pub const SEARCH_READ_BACKEND_MEILI: &str = "meili";
pub const SEARCH_WRITE_MODE_MEILI_ONLY: &str = "meili_only";
pub const SUGGEST_READ_BACKEND_LEGACY: &str = "legacy";
pub const SHADOW_SAMPLE_RATE_DISABLED: &str = "0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRuntimeFlags {
    pub search_read_backend: String,
    pub search_write_mode: String,
    pub suggest_read_backend: String,
    pub shadow_sample_rate: String,
}

impl Default for SearchRuntimeFlags {
    fn default() -> Self {
        Self {
            search_read_backend: SEARCH_READ_BACKEND_MEILI.to_string(),
            search_write_mode: SEARCH_WRITE_MODE_MEILI_ONLY.to_string(),
            suggest_read_backend: SUGGEST_READ_BACKEND_LEGACY.to_string(),
            shadow_sample_rate: SHADOW_SAMPLE_RATE_DISABLED.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct SearchRuntimeFlagsHandle {
    state: Arc<RwLock<SearchRuntimeFlags>>,
}

impl SearchRuntimeFlagsHandle {
    pub async fn get(&self) -> SearchRuntimeFlags {
        self.state.read().await.clone()
    }
}

pub async fn watch_search_runtime_flags(
    pool: Pool<Postgres>,
    poll_interval: Duration,
    service: &'static str,
) -> Result<SearchRuntimeFlagsHandle> {
    let initial = load_search_runtime_flags(&pool).await?;
    log_search_runtime_flags(service, "startup", &initial);

    let state = Arc::new(RwLock::new(initial));
    let state_ref = Arc::clone(&state);

    tokio::spawn(async move {
        let poll_interval = if poll_interval.is_zero() {
            Duration::from_secs(1)
        } else {
            poll_interval
        };

        let mut poll_timer = time::interval(poll_interval);
        poll_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
        poll_timer.tick().await;

        loop {
            poll_timer.tick().await;
            match load_search_runtime_flags(&pool).await {
                Ok(next) => {
                    let mut guard = state_ref.write().await;
                    if *guard != next {
                        log_search_runtime_flags(service, "poll", &next);
                        *guard = next;
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        service = service,
                        error = %err,
                        "search runtime flags refresh failed"
                    );
                }
            }
        }
    });

    Ok(SearchRuntimeFlagsHandle { state })
}

pub async fn load_search_runtime_flags(pool: &Pool<Postgres>) -> Result<SearchRuntimeFlags> {
    let rows = match sqlx::query("SELECT flag_name, flag_value FROM cn_search.runtime_flags")
        .fetch_all(pool)
        .await
    {
        Ok(rows) => rows,
        Err(err) if is_missing_runtime_flags_table(&err) => {
            tracing::warn!(
                error = %err,
                "cn_search.runtime_flags is unavailable; using compatibility defaults"
            );
            return Ok(SearchRuntimeFlags::default());
        }
        Err(err) => return Err(err.into()),
    };

    let mut flags = SearchRuntimeFlags::default();
    for row in rows {
        let flag_name: String = row.try_get("flag_name")?;
        let flag_value: String = row.try_get("flag_value")?;
        apply_flag_value(&mut flags, &flag_name, &flag_value);
    }

    Ok(flags)
}

pub fn log_search_runtime_flags(service: &str, trigger: &str, flags: &SearchRuntimeFlags) {
    tracing::info!(
        service = service,
        trigger = trigger,
        search_read_backend = %flags.search_read_backend,
        search_write_mode = %flags.search_write_mode,
        suggest_read_backend = %flags.suggest_read_backend,
        shadow_sample_rate = %flags.shadow_sample_rate,
        "search runtime flags loaded"
    );
}

fn apply_flag_value(flags: &mut SearchRuntimeFlags, flag_name: &str, flag_value: &str) {
    let value = flag_value.trim();
    if value.is_empty() {
        return;
    }

    match flag_name.trim() {
        FLAG_SEARCH_READ_BACKEND => flags.search_read_backend = value.to_string(),
        FLAG_SEARCH_WRITE_MODE => flags.search_write_mode = value.to_string(),
        FLAG_SUGGEST_READ_BACKEND => flags.suggest_read_backend = value.to_string(),
        FLAG_SHADOW_SAMPLE_RATE => flags.shadow_sample_rate = value.to_string(),
        _ => {}
    }
}

fn is_missing_runtime_flags_table(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            matches!(db_err.code().as_deref(), Some("42P01") | Some("3F000"))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::collections::HashSet;
    use std::sync::OnceLock;
    use tokio::sync::{Mutex, OnceCell};

    static MIGRATIONS: OnceCell<()> = OnceCell::const_new();

    fn database_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://cn:cn_password@localhost:5432/cn".to_string())
    }

    fn db_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    async fn ensure_migrated(pool: &Pool<Postgres>) {
        MIGRATIONS
            .get_or_init(|| async {
                crate::migrations::run(pool)
                    .await
                    .expect("run community-node migrations");
            })
            .await;
    }

    async fn test_pool() -> Pool<Postgres> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url())
            .await
            .expect("connect database");
        ensure_migrated(&pool).await;
        pool
    }

    async fn seed_flags(
        pool: &Pool<Postgres>,
        search_read_backend: &str,
        shadow_sample_rate: &str,
    ) {
        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) VALUES ($1, $2, 'test') ON CONFLICT (flag_name) DO UPDATE SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(FLAG_SEARCH_READ_BACKEND)
        .bind(search_read_backend)
        .execute(pool)
        .await
        .expect("upsert search_read_backend");

        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) VALUES ($1, $2, 'test') ON CONFLICT (flag_name) DO UPDATE SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(FLAG_SEARCH_WRITE_MODE)
        .bind(SEARCH_WRITE_MODE_MEILI_ONLY)
        .execute(pool)
        .await
        .expect("upsert search_write_mode");

        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) VALUES ($1, $2, 'test') ON CONFLICT (flag_name) DO UPDATE SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(FLAG_SUGGEST_READ_BACKEND)
        .bind(SUGGEST_READ_BACKEND_LEGACY)
        .execute(pool)
        .await
        .expect("upsert suggest_read_backend");

        sqlx::query(
            "INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by) VALUES ($1, $2, 'test') ON CONFLICT (flag_name) DO UPDATE SET flag_value = EXCLUDED.flag_value, updated_at = NOW(), updated_by = EXCLUDED.updated_by",
        )
        .bind(FLAG_SHADOW_SAMPLE_RATE)
        .bind(shadow_sample_rate)
        .execute(pool)
        .await
        .expect("upsert shadow_sample_rate");
    }

    #[test]
    fn default_flags_are_backward_compatible_with_meili() {
        let flags = SearchRuntimeFlags::default();
        assert_eq!(flags.search_read_backend, SEARCH_READ_BACKEND_MEILI);
        assert_eq!(flags.search_write_mode, SEARCH_WRITE_MODE_MEILI_ONLY);
        assert_eq!(flags.suggest_read_backend, SUGGEST_READ_BACKEND_LEGACY);
        assert_eq!(flags.shadow_sample_rate, SHADOW_SAMPLE_RATE_DISABLED);
    }

    #[tokio::test]
    async fn load_search_runtime_flags_reads_seeded_defaults() {
        let _guard = db_test_lock().lock().await;
        let pool = test_pool().await;

        seed_flags(
            &pool,
            SEARCH_READ_BACKEND_MEILI,
            SHADOW_SAMPLE_RATE_DISABLED,
        )
        .await;

        let flags = load_search_runtime_flags(&pool)
            .await
            .expect("load search runtime flags");

        assert_eq!(flags.search_read_backend, SEARCH_READ_BACKEND_MEILI);
        assert_eq!(flags.search_write_mode, SEARCH_WRITE_MODE_MEILI_ONLY);
        assert_eq!(flags.suggest_read_backend, SUGGEST_READ_BACKEND_LEGACY);
        assert_eq!(flags.shadow_sample_rate, SHADOW_SAMPLE_RATE_DISABLED);
    }

    #[tokio::test]
    async fn migrations_enable_required_search_extensions() {
        let _guard = db_test_lock().lock().await;
        let pool = test_pool().await;

        let installed: Vec<String> =
            sqlx::query_scalar("SELECT extname FROM pg_extension WHERE extname = ANY($1::text[])")
                .bind(vec!["age", "pg_trgm", "pgroonga"])
                .fetch_all(&pool)
                .await
                .expect("query installed extensions");

        let installed_set: HashSet<String> = installed.into_iter().collect();
        assert!(installed_set.contains("age"));
        assert!(installed_set.contains("pg_trgm"));
        assert!(installed_set.contains("pgroonga"));
    }

    #[tokio::test]
    async fn load_search_runtime_flags_reads_runtime_overrides() {
        let _guard = db_test_lock().lock().await;
        let pool = test_pool().await;

        seed_flags(&pool, "pg", "25").await;

        let flags = load_search_runtime_flags(&pool)
            .await
            .expect("load search runtime flags");

        assert_eq!(flags.search_read_backend, "pg");
        assert_eq!(flags.search_write_mode, SEARCH_WRITE_MODE_MEILI_ONLY);
        assert_eq!(flags.suggest_read_backend, SUGGEST_READ_BACKEND_LEGACY);
        assert_eq!(flags.shadow_sample_rate, "25");

        seed_flags(
            &pool,
            SEARCH_READ_BACKEND_MEILI,
            SHADOW_SAMPLE_RATE_DISABLED,
        )
        .await;
    }
}
