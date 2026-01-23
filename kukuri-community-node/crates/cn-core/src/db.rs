use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub async fn connect(database_url: &str) -> Result<Pool<Postgres>> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn check_ready(pool: &Pool<Postgres>) -> Result<()> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
