use super::*;

pub(crate) static STORE_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

impl SqliteStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = sqlite_pool_options(
            if database_url.contains(":memory:") {
                1
            } else {
                4
            },
            !database_url.contains(":memory:"),
        )
        .connect(database_url)
        .await
        .with_context(|| format!("failed to connect sqlite database: {database_url}"))?;

        run_store_migrations(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn connect_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true);
        let pool = sqlite_pool_options(4, true)
            .connect_with(options)
            .await
            .with_context(|| format!("failed to connect sqlite database: {}", path.display()))?;

        run_store_migrations(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn connect_memory() -> Result<Self> {
        Self::connect("sqlite::memory:").await
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

fn sqlite_pool_options(max_connections: u32, enable_wal: bool) -> SqlitePoolOptions {
    SqlitePoolOptions::new()
        .min_connections(1)
        .max_connections(max_connections)
        .after_connect(move |connection, _meta| {
            Box::pin(async move {
                sqlx::query("PRAGMA busy_timeout = 5000")
                    .execute(&mut *connection)
                    .await?;
                sqlx::query("PRAGMA synchronous = NORMAL")
                    .execute(&mut *connection)
                    .await?;
                if enable_wal {
                    sqlx::query("PRAGMA journal_mode = WAL")
                        .execute(&mut *connection)
                        .await?;
                }
                Ok(())
            })
        })
}

async fn run_store_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    repair_line_ending_only_migration_checksums(pool).await?;
    STORE_MIGRATOR.run(pool).await?;
    Ok(())
}

async fn repair_line_ending_only_migration_checksums(pool: &Pool<Sqlite>) -> Result<()> {
    if !sqlite_table_exists(pool, "_sqlx_migrations").await? {
        return Ok(());
    }

    let applied_migrations = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await?;

    for (version, applied_checksum) in applied_migrations {
        let Some(migration) = STORE_MIGRATOR.iter().find(|migration| {
            migration.version == version && !migration.migration_type.is_down_migration()
        }) else {
            continue;
        };

        if applied_checksum.as_slice() == migration.checksum.as_ref() {
            continue;
        }

        if checksum_matches_line_ending_variant(&applied_checksum, migration.sql.as_ref()) {
            repair_migration_checksum(pool, version).await?;
        }
    }

    Ok(())
}

async fn repair_migration_checksum(pool: &Pool<Sqlite>, version: i64) -> Result<()> {
    let migration = STORE_MIGRATOR
        .iter()
        .find(|migration| {
            migration.version == version && !migration.migration_type.is_down_migration()
        })
        .with_context(|| format!("embedded migration {version} is missing"))?;
    let result = sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = ?2")
        .bind(migration.checksum.as_ref())
        .bind(version)
        .execute(pool)
        .await?;
    if result.rows_affected() != 1 {
        anyhow::bail!("expected to repair one migration row for version {version}");
    }
    Ok(())
}

async fn sqlite_table_exists(pool: &Pool<Sqlite>, name: &str) -> Result<bool> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?
    .is_some();
    Ok(exists)
}

fn checksum_matches_line_ending_variant(applied_checksum: &[u8], sql: &str) -> bool {
    let lf_sql = normalize_sql_line_endings(sql);
    let lf_checksum = migration_checksum(lf_sql.as_str());
    if applied_checksum == lf_checksum {
        return true;
    }

    let crlf_sql = lf_sql.replace('\n', "\r\n");
    let crlf_checksum = migration_checksum(crlf_sql.as_str());
    applied_checksum == crlf_checksum
}

#[cfg(test)]
pub(crate) fn alternate_line_ending_checksum(
    sql: &str,
    current_checksum: &[u8],
) -> Option<Vec<u8>> {
    let lf_sql = normalize_sql_line_endings(sql);
    let lf_checksum = migration_checksum(lf_sql.as_str());
    if lf_checksum != current_checksum {
        return Some(lf_checksum);
    }

    let crlf_sql = lf_sql.replace('\n', "\r\n");
    let crlf_checksum = migration_checksum(crlf_sql.as_str());
    if crlf_checksum != current_checksum {
        return Some(crlf_checksum);
    }

    None
}

fn normalize_sql_line_endings(sql: &str) -> String {
    sql.replace("\r\n", "\n").replace('\r', "\n")
}

fn migration_checksum(sql: &str) -> Vec<u8> {
    Vec::from(Sha384::digest(sql.as_bytes()).as_slice())
}
