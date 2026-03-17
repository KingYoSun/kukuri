use anyhow::Result;
use kukuri_cn_core::{
    DatabaseInitMode, TestDatabase, connect_postgres, initialize_database,
    initialize_database_for_runtime,
};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:55432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    let enabled = std::env::var("KUKURI_CN_RUN_INTEGRATION_TESTS")
        .ok()
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);
    if !enabled {
        return None;
    }
    Some(
        std::env::var("COMMUNITY_NODE_DATABASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ADMIN_DATABASE_URL.to_string()),
    )
}

#[tokio::test]
async fn require_ready_accepts_prepared_database() -> Result<()> {
    let Some(admin_database_url) = integration_test_admin_database_url() else {
        eprintln!("skipping cn-core integration test; set KUKURI_CN_RUN_INTEGRATION_TESTS=1");
        return Ok(());
    };
    let database =
        TestDatabase::create(admin_database_url.as_str(), "cn_core_runtime_ready").await?;
    let pool = connect_postgres(database.database_url.as_str()).await?;

    initialize_database(&pool).await?;
    let result = initialize_database_for_runtime(&pool, DatabaseInitMode::RequireReady).await;

    database.cleanup().await?;
    result
}
