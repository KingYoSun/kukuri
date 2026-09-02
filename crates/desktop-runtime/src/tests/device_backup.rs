use super::*;

use std::collections::BTreeMap;

use kukuri_store::SqliteStore;

use crate::accounts::{ensure_accounts_initialized, list_accounts};
use crate::backup::{
    CreateDeviceBackupRequest, DeviceBackupCancellation, PreviewDeviceBackupRequest,
    RestoreDeviceBackupRequest, commit_device_restore, create_device_backup,
    finalize_device_restore, install_prepared_device_restore, prepare_device_restore,
    preview_device_backup, recover_interrupted_restore,
};
use crate::identity::{IdentityStorageMode, load_existing_keys};

const PASSPHRASE: &str = "correct horse battery staple";

async fn initialized_app_data() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("tempdir");
    let db_path = ensure_accounts_initialized(dir.path(), IdentityStorageMode::FileOnly)
        .expect("initialize accounts");
    let store = SqliteStore::connect_file(&db_path)
        .await
        .expect("initialize sqlite");
    store.close().await;
    (dir, db_path)
}

#[tokio::test]
async fn encrypted_device_backup_restores_one_account_as_one_file() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let source_keys = load_existing_keys(&source_db, IdentityStorageMode::FileOnly)
        .expect("load source identity")
        .expect("source identity");
    let iroh_dir = source_db.with_extension("iroh-data");
    fs::create_dir_all(iroh_dir.join("blobs")).expect("create iroh data");
    fs::write(iroh_dir.join("blobs").join("marker.bin"), b"portable blob")
        .expect("write portable marker");
    fs::write(iroh_dir.join("endpoint-secret.json"), b"device-bound")
        .expect("write endpoint secret");

    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("account.kukuri-backup");
    let frontend_state = BTreeMap::from([
        ("kukuri.desktop.locale".to_string(), "\"ja\"".to_string()),
        (
            "kukuri.workspace.layout".to_string(),
            "{\"version\":1}".to_string(),
        ),
    ]);
    let summary = create_device_backup(
        source.path(),
        &source_db,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: frontend_state.clone(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("create device backup");
    assert!(archive_path.is_file());
    assert!(summary.bytes > 0);
    assert_eq!(summary.public_key, source_keys.public_key_hex());

    let (target, _target_db) = initialized_app_data().await;
    let preview = preview_device_backup(
        target.path(),
        &PreviewDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
        },
    )
    .expect("preview device backup");
    assert_eq!(preview.public_key, source_keys.public_key_hex());
    assert!(preview.existing_account_id.is_none());
    assert!(
        preview
            .requires_reconsent
            .contains(&"app_legal_documents".to_string())
    );

    let prepared = prepare_device_restore(
        target.path(),
        &RestoreDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            replace_existing: false,
            apply_frontend_state: true,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("prepare device restore");
    assert_eq!(
        fs::read(
            prepared
                .staging_db_path()
                .with_extension("iroh-data")
                .join("blobs")
                .join("marker.bin")
        )
        .expect("read staged marker"),
        b"portable blob"
    );
    assert!(
        !prepared
            .staging_db_path()
            .with_extension("iroh-data")
            .join("endpoint-secret.json")
            .exists()
    );
    let staged_store = SqliteStore::connect_file(prepared.staging_db_path())
        .await
        .expect("validate staged sqlite");
    staged_store.close().await;

    let installed =
        install_prepared_device_restore(target.path(), prepared).expect("install restored account");
    let restored_db = installed.db_path();
    let result = commit_device_restore(&installed).expect("commit restored account");
    assert_eq!(result.frontend_state, frontend_state);
    finalize_device_restore(installed).expect("finalize restored account");

    let restored_keys = load_existing_keys(&restored_db, IdentityStorageMode::FileOnly)
        .expect("load restored identity")
        .expect("restored identity");
    assert_eq!(restored_keys.public_key_hex(), source_keys.public_key_hex());
    let snapshot = list_accounts(target.path()).expect("list restored accounts");
    assert_eq!(snapshot.active_account_id, result.account.id);
    assert_eq!(snapshot.accounts.len(), 2);
}

#[tokio::test]
async fn restore_failures_preserve_the_existing_account_registry() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (app_data, db_path) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("account.kukuri-backup");
    create_device_backup(
        app_data.path(),
        &db_path,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::new(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("create backup");
    let before = list_accounts(app_data.path()).expect("list accounts before failure");

    let wrong_passphrase = prepare_device_restore(
        app_data.path(),
        &RestoreDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: "definitely wrong".to_string(),
            replace_existing: true,
            apply_frontend_state: false,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    );
    assert!(wrong_passphrase.is_err());
    assert_eq!(
        list_accounts(app_data.path()).expect("list accounts after wrong passphrase"),
        before
    );

    let replacement_without_confirmation = prepare_device_restore(
        app_data.path(),
        &RestoreDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            replace_existing: false,
            apply_frontend_state: false,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    );
    assert!(replacement_without_confirmation.is_err());
    assert_eq!(
        list_accounts(app_data.path()).expect("list accounts after rejected replacement"),
        before
    );
}

#[tokio::test]
async fn interrupted_replacement_is_rolled_back_on_recovery() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (app_data, db_path) = initialized_app_data().await;
    let before = list_accounts(app_data.path()).expect("list accounts before replacement");
    let original_identity = load_existing_keys(&db_path, IdentityStorageMode::FileOnly)
        .expect("load original identity")
        .expect("original identity")
        .public_key_hex();
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("replacement.kukuri-backup");
    create_device_backup(
        app_data.path(),
        &db_path,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::new(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("create replacement backup");
    let prepared = prepare_device_restore(
        app_data.path(),
        &RestoreDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            replace_existing: true,
            apply_frontend_state: false,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("prepare replacement");
    let installed = install_prepared_device_restore(app_data.path(), prepared)
        .expect("install uncommitted replacement");
    drop(installed);

    recover_interrupted_restore(app_data.path()).expect("recover interrupted replacement");
    assert_eq!(
        list_accounts(app_data.path()).expect("list accounts after recovery"),
        before
    );
    let recovered_identity = load_existing_keys(&db_path, IdentityStorageMode::FileOnly)
        .expect("load recovered identity")
        .expect("recovered identity")
        .public_key_hex();
    assert_eq!(recovered_identity, original_identity);
}
