use anyhow::Result;
use sqlx::{migrate::Migrator, Pool, Postgres};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

pub async fn run(pool: &Pool<Postgres>) -> Result<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}
