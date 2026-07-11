use anyhow::Result;
use kukuri_cn_core::{
    DatabaseInitMode, TestDatabase, connect_postgres, initialize_database,
    initialize_database_for_runtime,
};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:15432/cn";

fn integration_test_admin_database_url() -> Option<String> {
    kukuri_test_support::gated_env_url(
        "KUKURI_CN_RUN_INTEGRATION_TESTS",
        "COMMUNITY_NODE_DATABASE_URL",
        DEFAULT_ADMIN_DATABASE_URL,
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
