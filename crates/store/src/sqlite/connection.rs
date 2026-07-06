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

// checksum 不一致(CRLF checkout 由来を含む)はここで失敗し、起動 Failed 画面で
// DatabaseMigration として通知される。かつての接続毎 CRLF 自己修復(#211)は
// WP-C6 で撤去済み — 根本原因は .gitattributes (*.sql text eol=lf、#211 同梱)で解消済み。
async fn run_store_migrations(pool: &Pool<Sqlite>) -> Result<()> {
    STORE_MIGRATOR.run(pool).await?;
    Ok(())
}
