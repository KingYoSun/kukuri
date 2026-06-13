use super::*;
use anyhow::Result;
use sqlx::Row;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;

const PRE_METAVERSE_FIXTURE: &str =
    include_str!("../../fixtures/sqlite/pre-metaverse-game-room-columns.fixture");

#[tokio::test]
async fn connect_file_repairs_line_ending_only_migration_checksum_mismatches() {
    let tempdir = tempdir().expect("tempdir");
    let db_path = tempdir.path().join("store.db");
    let store = SqliteStore::connect_file(&db_path)
        .await
        .expect("initialize sqlite store");
    store.close().await;

    let database_url = format!("sqlite://{}", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("reopen sqlite db");
    for version in [20260319000000_i64, 20260319010000_i64] {
        let migration = STORE_MIGRATOR
            .iter()
            .find(|migration| {
                migration.version == version && !migration.migration_type.is_down_migration()
            })
            .expect("embedded store migration");
        let alternate_checksum =
            alternate_line_ending_checksum(migration.sql.as_ref(), migration.checksum.as_ref())
                .expect("alternate line-ending checksum");
        sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = ?2")
            .bind(alternate_checksum)
            .bind(version)
            .execute(&pool)
            .await
            .expect("rewrite migration checksum to alternate line ending");
    }
    pool.close().await;

    let reopened = SqliteStore::connect_file(&db_path)
        .await
        .expect("reopen store after repairing line-ending-only migration checksum mismatch");
    for version in [20260319000000_i64, 20260319010000_i64] {
        let stored_checksum = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT checksum FROM _sqlx_migrations WHERE version = ?1",
        )
        .bind(version)
        .fetch_one(reopened.pool())
        .await
        .expect("load repaired checksum");
        let expected_checksum = STORE_MIGRATOR
            .iter()
            .find(|migration| {
                migration.version == version && !migration.migration_type.is_down_migration()
            })
            .expect("embedded store migration")
            .checksum
            .to_vec();

        assert_eq!(stored_checksum, expected_checksum);
    }
}

#[tokio::test]
async fn connect_file_applies_unfiltered_projection_indexes() {
    let tempdir = tempdir().expect("tempdir");
    let db_path = tempdir.path().join("store.db");
    let expected_indexes = [
        "idx_game_room_cache_topic_updated_all",
        "idx_live_session_cache_topic_started_all",
        "idx_object_index_cache_topic_created_all",
        "idx_object_thread_cache_topic_root_created_all",
    ];

    let store = SqliteStore::connect_file(&db_path)
        .await
        .expect("initialize sqlite store");
    let mut actual_indexes = sqlx::query_scalar::<_, String>(
        r#"
        SELECT name
        FROM sqlite_master
        WHERE type = 'index'
          AND name IN (
            'idx_game_room_cache_topic_updated_all',
            'idx_live_session_cache_topic_started_all',
            'idx_object_index_cache_topic_created_all',
            'idx_object_thread_cache_topic_root_created_all'
          )
        ORDER BY name
        "#,
    )
    .fetch_all(store.pool())
    .await
    .expect("load unfiltered projection indexes");
    actual_indexes.sort();
    assert_eq!(actual_indexes, expected_indexes);
    store.close().await;

    let reopened = SqliteStore::connect_file(&db_path)
        .await
        .expect("reopen sqlite store");
    let mut reopened_indexes = sqlx::query_scalar::<_, String>(
        r#"
        SELECT name
        FROM sqlite_master
        WHERE type = 'index'
          AND name IN (
            'idx_game_room_cache_topic_updated_all',
            'idx_live_session_cache_topic_started_all',
            'idx_object_index_cache_topic_created_all',
            'idx_object_thread_cache_topic_root_created_all'
          )
        ORDER BY name
        "#,
    )
    .fetch_all(reopened.pool())
    .await
    .expect("load unfiltered projection indexes after reopen");
    reopened_indexes.sort();
    assert_eq!(reopened_indexes, expected_indexes);
}

#[tokio::test]
async fn connect_file_migrates_pre_metaverse_game_room_fixture() {
    let tempdir = tempdir().expect("tempdir");
    let db_path = tempdir.path().join("pre-metaverse.db");
    let applied_through = fixture_applied_through(PRE_METAVERSE_FIXTURE);
    materialize_sqlite_fixture(&db_path, applied_through)
        .await
        .expect("materialize old sqlite fixture");

    let database_url = format!("sqlite://{}", db_path.display());
    let pre_migration_pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("open old sqlite fixture");
    assert!(
        !sqlite_column_exists(&pre_migration_pool, "game_room_cache", "room_kind")
            .await
            .expect("check old fixture room_kind column"),
        "fixture should represent the pre-metaverse game_room_cache schema"
    );
    pre_migration_pool.close().await;

    let migrated = SqliteStore::connect_file(&db_path)
        .await
        .expect("migrate old sqlite fixture");

    assert!(
        sqlite_column_exists(migrated.pool(), "game_room_cache", "room_kind")
            .await
            .expect("check migrated room_kind column")
    );
    assert!(
        sqlite_column_exists(migrated.pool(), "game_room_cache", "metaverse_json")
            .await
            .expect("check migrated metaverse_json column")
    );
    let index_exists = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT 1
        FROM sqlite_master
        WHERE type = 'index'
          AND name = 'idx_game_room_cache_topic_kind_updated'
        LIMIT 1
        "#,
    )
    .fetch_optional(migrated.pool())
    .await
    .expect("check migrated game room kind index")
    .is_some();
    assert!(index_exists);

    let latest_migration = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM _sqlx_migrations WHERE version = ?1 AND success = true",
    )
    .bind(20260527000000_i64)
    .fetch_optional(migrated.pool())
    .await
    .expect("check latest migration record")
    .is_some();
    assert!(latest_migration);
}

fn fixture_applied_through(fixture: &str) -> i64 {
    fixture
        .lines()
        .find_map(|line| line.strip_prefix("applied_through="))
        .expect("fixture applied_through")
        .parse()
        .expect("fixture applied_through version")
}

async fn materialize_sqlite_fixture(db_path: &std::path::Path, applied_through: i64) -> Result<()> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success BOOLEAN NOT NULL,
            checksum BLOB NOT NULL,
            execution_time BIGINT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    for migration in STORE_MIGRATOR
        .iter()
        .filter(|migration| {
            migration.version <= applied_through && !migration.migration_type.is_down_migration()
        })
        .collect::<Vec<_>>()
    {
        sqlx::raw_sql(migration.sql.as_ref()).execute(&pool).await?;
        sqlx::query(
            r#"
            INSERT INTO _sqlx_migrations (
                version,
                description,
                success,
                checksum,
                execution_time
            )
            VALUES (?1, ?2, true, ?3, 0)
            "#,
        )
        .bind(migration.version)
        .bind(migration.description.as_ref())
        .bind(migration.checksum.as_ref())
        .execute(&pool)
        .await?;
    }

    pool.close().await;
    Ok(())
}

async fn sqlite_column_exists(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    table_name: &str,
    column_name: &str,
) -> Result<bool> {
    let rows = sqlx::query("SELECT name FROM pragma_table_info(?1)")
        .bind(table_name)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column_name))
}
