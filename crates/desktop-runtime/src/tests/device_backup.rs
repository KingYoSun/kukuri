use super::*;

use std::collections::BTreeMap;

use kukuri_store::SqliteStore;

use crate::accounts::{
    account_db_path, account_id_for_pubkey, ensure_accounts_initialized, list_accounts,
};
use crate::backup::{
    CreateDeviceBackupRequest, DeviceBackupCancellation, DeviceRestorePhase,
    DeviceRestoreTestFailurePoint, PreviewDeviceBackupRequest, RestoreDeviceBackupRequest,
    acknowledge_pending_device_restore_frontend_state, commit_device_restore, create_device_backup,
    fail_device_backup_writes_after, fail_device_restore_at, finalize_device_restore,
    install_prepared_device_restore, install_prepared_device_restore_with_keyring,
    mark_device_restore_activated, mark_device_restore_awaiting_consent,
    pending_device_restore_frontend_state, pending_device_restore_phase, prepare_device_restore,
    preview_device_backup, recover_interrupted_restore, validate_prepared_device_restore,
};
use crate::community_node::{
    COMMUNITY_NODE_CONSENT_PURPOSE, COMMUNITY_NODE_INVITE_CODE_PURPOSE,
    COMMUNITY_NODE_TOKEN_PURPOSE, CommunityNodeConfig, CommunityNodeNodeConfig,
    load_community_node_config_from_file, save_community_node_config,
};
use crate::host::{
    DesiredSubscription, DesiredSubscriptionScope, load_desired_subscriptions,
    save_desired_subscriptions,
};
use crate::identity::{
    IdentityStorageMode, KeyringStore, load_existing_keys, load_optional_secret,
    load_optional_secret_with_keyring, persist_optional_secret,
    persist_optional_secret_with_keyring,
};
use crate::runtime::{
    GOSSIP_SUBSCRIPTION_STATE_KEY, GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
    PRIVATE_CHANNEL_CAPABILITIES_KEY, PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
};

const PASSPHRASE: &str = "correct horse battery staple";

#[derive(Default)]
struct FakeDeviceBackupKeyring {
    entries: std::sync::Mutex<BTreeMap<(String, String), String>>,
}

impl KeyringStore for FakeDeviceBackupKeyring {
    fn get_password(&self, service: &str, account: &str) -> anyhow::Result<Option<String>> {
        Ok(self
            .entries
            .lock()
            .expect("fake keyring lock")
            .get(&(service.to_string(), account.to_string()))
            .cloned())
    }

    fn set_password(&self, service: &str, account: &str, secret: &str) -> anyhow::Result<()> {
        self.entries.lock().expect("fake keyring lock").insert(
            (service.to_string(), account.to_string()),
            secret.to_string(),
        );
        Ok(())
    }

    fn delete_password(&self, service: &str, account: &str) -> anyhow::Result<()> {
        self.entries
            .lock()
            .expect("fake keyring lock")
            .remove(&(service.to_string(), account.to_string()));
        Ok(())
    }
}

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

fn app_data_snapshot(root: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    fn collect(
        root: &std::path::Path,
        current: &std::path::Path,
        files: &mut BTreeMap<String, Vec<u8>>,
    ) {
        let mut entries = fs::read_dir(current)
            .expect("read snapshot directory")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("collect snapshot directory");
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let file_type = entry.file_type().expect("snapshot file type");
            if file_type.is_dir() {
                collect(root, &entry.path(), files);
            } else if file_type.is_file() {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .expect("snapshot relative path")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.insert(relative, fs::read(entry.path()).expect("snapshot file"));
            }
        }
    }

    let mut files = BTreeMap::new();
    collect(root, root, &mut files);
    files
}

fn archive_with_header_version(mut bytes: Vec<u8>, replacement: u8) -> Vec<u8> {
    let needle = b"\"version\":1";
    let index = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("backup header version");
    bytes[index + needle.len() - 1] = replacement;
    bytes
}

fn create_restore_fixture(
    app_data_dir: &std::path::Path,
    db_path: &std::path::Path,
    archive_path: &std::path::Path,
) {
    create_device_backup(
        app_data_dir,
        db_path,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::new(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("create restore fixture");
}

fn prepare_restore_fixture(
    app_data_dir: &std::path::Path,
    archive_path: &std::path::Path,
    replace_existing: bool,
) -> crate::backup::PreparedDeviceRestore {
    prepare_device_restore(
        app_data_dir,
        &RestoreDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            replace_existing,
            apply_frontend_state: false,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("prepare restore fixture")
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
    let node_base_url = "https://backup-node.example";
    save_community_node_config(
        &source_db,
        &CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: node_base_url.to_string(),
                resolved_urls: None,
            }],
        },
    )
    .expect("persist community node config");
    let desired_subscription = DesiredSubscription {
        topic: "kukuri:topic:backup-subscription".to_string(),
        scope: DesiredSubscriptionScope::Public,
    };
    save_desired_subscriptions(&source_db, std::slice::from_ref(&desired_subscription))
        .expect("persist desired subscription fixture");
    for (purpose, key, value) in [
        (
            PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
            PRIVATE_CHANNEL_CAPABILITIES_KEY,
            "private-channel-capability",
        ),
        (
            GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
            GOSSIP_SUBSCRIPTION_STATE_KEY,
            "gossip-subscription",
        ),
        (
            COMMUNITY_NODE_INVITE_CODE_PURPOSE,
            node_base_url,
            "portable-invite",
        ),
        (
            COMMUNITY_NODE_TOKEN_PURPOSE,
            node_base_url,
            "device-session-token",
        ),
        (
            COMMUNITY_NODE_CONSENT_PURPOSE,
            node_base_url,
            "device-consent-record",
        ),
    ] {
        persist_optional_secret(
            &source_db,
            IdentityStorageMode::FileOnly,
            purpose,
            key,
            value,
        )
        .expect("persist backup secret fixture");
    }

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
    mark_device_restore_awaiting_consent(target.path()).expect("mark awaiting app consent");
    mark_device_restore_activated(target.path()).expect("mark restored runtime activated");
    finalize_device_restore(installed).expect("finalize restored account");

    let restored_keys = load_existing_keys(&restored_db, IdentityStorageMode::FileOnly)
        .expect("load restored identity")
        .expect("restored identity");
    assert_eq!(restored_keys.public_key_hex(), source_keys.public_key_hex());
    assert_eq!(
        load_desired_subscriptions(&restored_db).expect("load restored desired subscriptions"),
        vec![desired_subscription]
    );
    let snapshot = list_accounts(target.path()).expect("list restored accounts");
    assert_eq!(snapshot.active_account_id, result.account.id);
    assert_eq!(snapshot.accounts.len(), 2);
    assert_eq!(
        load_community_node_config_from_file(&restored_db)
            .expect("load restored community node config")
            .expect("restored community node config"),
        CommunityNodeConfig {
            nodes: vec![CommunityNodeNodeConfig {
                base_url: node_base_url.to_string(),
                resolved_urls: None,
            }],
        }
    );
    for (purpose, key, expected) in [
        (
            PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
            PRIVATE_CHANNEL_CAPABILITIES_KEY,
            Some("private-channel-capability"),
        ),
        (
            GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
            GOSSIP_SUBSCRIPTION_STATE_KEY,
            Some("gossip-subscription"),
        ),
        (
            COMMUNITY_NODE_INVITE_CODE_PURPOSE,
            node_base_url,
            Some("portable-invite"),
        ),
        (COMMUNITY_NODE_TOKEN_PURPOSE, node_base_url, None),
        (COMMUNITY_NODE_CONSENT_PURPOSE, node_base_url, None),
    ] {
        assert_eq!(
            load_optional_secret(&restored_db, IdentityStorageMode::FileOnly, purpose, key)
                .expect("load restored optional secret")
                .as_deref(),
            expected,
            "unexpected restored value for {purpose}/{key}"
        );
    }
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
    let state_before = app_data_snapshot(app_data.path());

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
    assert_eq!(app_data_snapshot(app_data.path()), state_before);

    let original = fs::read(&archive_path).expect("read backup for rejection fixtures");
    let corrupt_path = archive_dir.path().join("corrupt.kukuri-backup");
    let mut corrupt = original.clone();
    let corrupt_index = corrupt.len() - 8;
    corrupt[corrupt_index] ^= 0x80;
    fs::write(&corrupt_path, corrupt).expect("write corrupt backup");
    let truncated_path = archive_dir.path().join("truncated.kukuri-backup");
    fs::write(&truncated_path, &original[..original.len() - 6]).expect("write truncated backup");
    let unknown_path = archive_dir.path().join("unknown-version.kukuri-backup");
    fs::write(&unknown_path, archive_with_header_version(original, b'9'))
        .expect("write unknown-version backup");

    for rejected_path in [&corrupt_path, &truncated_path, &unknown_path] {
        let rejected = prepare_device_restore(
            app_data.path(),
            &RestoreDeviceBackupRequest {
                path: rejected_path.display().to_string(),
                passphrase: PASSPHRASE.to_string(),
                replace_existing: true,
                apply_frontend_state: false,
            },
            &DeviceBackupCancellation::default(),
            |_| {},
        );
        assert!(
            rejected.is_err(),
            "{} was accepted",
            rejected_path.display()
        );
        assert_eq!(
            app_data_snapshot(app_data.path()),
            state_before,
            "{} changed existing app data",
            rejected_path.display()
        );
    }

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
    assert_eq!(app_data_snapshot(app_data.path()), state_before);
}

#[tokio::test]
async fn canceled_backup_removes_partial_output_and_preserves_source_state() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let iroh_dir = source_db.with_extension("iroh-data");
    fs::create_dir_all(iroh_dir.join("blobs")).expect("create iroh blobs");
    fs::write(
        iroh_dir.join("blobs").join("large.bin"),
        vec![7u8; 256 * 1024],
    )
    .expect("write large portable blob");
    let state_before = app_data_snapshot(source.path());
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("canceled.kukuri-backup");
    let partial_path = archive_dir.path().join(".canceled.kukuri-backup.partial");
    let cancellation = DeviceBackupCancellation::default();
    let result = create_device_backup(
        source.path(),
        &source_db,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::new(),
        },
        &cancellation,
        |progress| {
            if progress.phase == crate::backup::DeviceBackupPhase::Encrypting {
                cancellation.cancel();
            }
        },
    );
    assert!(
        result
            .expect_err("canceled backup unexpectedly succeeded")
            .to_string()
            .contains("canceled")
    );
    assert!(!archive_path.exists());
    assert!(!partial_path.exists());
    assert_eq!(app_data_snapshot(source.path()), state_before);
}

#[tokio::test]
async fn existing_backup_destination_is_never_overwritten() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("existing.kukuri-backup");
    fs::write(&archive_path, b"existing backup").expect("write existing destination");

    let result = create_device_backup(
        source.path(),
        &source_db,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::new(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(&archive_path).expect("read protected destination"),
        b"existing backup"
    );
}

#[tokio::test]
async fn storage_exhaustion_during_backup_removes_partial_output() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let state_before = app_data_snapshot(source.path());
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("storage-full.kukuri-backup");
    let partial_path = archive_dir
        .path()
        .join(".storage-full.kukuri-backup.partial");
    let _failure = fail_device_backup_writes_after(1);

    let error = create_device_backup(
        source.path(),
        &source_db,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::new(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect_err("storage exhaustion unexpectedly succeeded");
    assert!(error.to_string().contains("failed to write backup magic"));
    assert!(!archive_path.exists());
    assert!(!partial_path.exists());
    assert_eq!(app_data_snapshot(source.path()), state_before);
}

#[tokio::test]
async fn storage_exhaustion_during_restore_preserves_existing_state() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir
        .path()
        .join("restore-storage-full.kukuri-backup");
    create_device_backup(
        source.path(),
        &source_db,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::new(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("create restore fixture");

    let (target, _target_db) = initialized_app_data().await;
    let state_before = app_data_snapshot(target.path());
    let _failure = fail_device_backup_writes_after(1);
    let error = match prepare_device_restore(
        target.path(),
        &RestoreDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            replace_existing: false,
            apply_frontend_state: false,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    ) {
        Ok(_) => panic!("restore storage exhaustion unexpectedly succeeded"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("failed to restore device backup entry")
    );
    assert_eq!(app_data_snapshot(target.path()), state_before);
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

mod recovery;
