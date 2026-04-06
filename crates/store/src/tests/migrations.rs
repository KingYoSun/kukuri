use super::*;

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
