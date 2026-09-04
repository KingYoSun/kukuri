use super::*;

#[tokio::test]
async fn committed_restore_remains_pending_until_consent_and_activation() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("pending-consent.kukuri-backup");
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
    .expect("create pending-consent fixture");

    let (target, _target_db) = initialized_app_data().await;
    let prepared = prepare_device_restore(
        target.path(),
        &RestoreDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            replace_existing: false,
            apply_frontend_state: false,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("prepare pending-consent restore");
    let installed = install_prepared_device_restore(target.path(), prepared)
        .expect("install pending-consent restore");
    commit_device_restore(&installed).expect("commit restored registry");

    let mut legacy_compatible_journal: serde_json::Value = serde_json::from_slice(
        &fs::read(target.path().join("device-restore-journal.json"))
            .expect("read committed restore journal"),
    )
    .expect("decode committed restore journal");
    assert_eq!(legacy_compatible_journal["version"], 1);
    assert_eq!(legacy_compatible_journal["phase"], "committed");
    legacy_compatible_journal
        .as_object_mut()
        .expect("restore journal object")
        .remove("frontend_state");
    fs::write(
        target.path().join("device-restore-journal.json"),
        serde_json::to_vec(&legacy_compatible_journal).expect("encode legacy journal"),
    )
    .expect("write legacy journal without frontend state");
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("read committed phase"),
        Some(DeviceRestorePhase::Committed)
    );
    recover_interrupted_restore(target.path()).expect("recover committed restore");
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("read preserved committed phase"),
        Some(DeviceRestorePhase::Committed)
    );

    mark_device_restore_awaiting_consent(target.path()).expect("mark awaiting consent");
    recover_interrupted_restore(target.path()).expect("recover consent-pending restore");
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("read awaiting-consent phase"),
        Some(DeviceRestorePhase::AwaitingConsent)
    );

    mark_device_restore_activated(target.path()).expect("mark activated");
    recover_interrupted_restore(target.path()).expect("finalize activated restore");
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("read finalized phase"),
        None
    );
    assert_eq!(
        pending_device_restore_frontend_state(target.path()).expect("legacy frontend marker"),
        Some(BTreeMap::new())
    );
    acknowledge_pending_device_restore_frontend_state(target.path())
        .expect("acknowledge legacy frontend marker");
}

#[tokio::test]
async fn registry_commit_stop_recovers_old_active_account_before_path_resolution() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("registry-cutpoint.kukuri-backup");
    create_restore_fixture(source.path(), &source_db, &archive_path);

    let (target, old_db) = initialized_app_data().await;
    let registry_before = list_accounts(target.path()).expect("old account registry");
    let prepared = prepare_restore_fixture(target.path(), &archive_path, false);
    let installed =
        install_prepared_device_restore(target.path(), prepared).expect("install restored account");
    let restored_db = installed.db_path();
    assert_ne!(restored_db, old_db);

    let failure = fail_device_restore_at(DeviceRestoreTestFailurePoint::AfterRegistryCommit);
    let error =
        commit_device_restore(&installed).expect_err("simulated process stop must fail commit");
    assert!(error.to_string().contains("simulated process stop"));
    drop(failure);
    let interrupted_registry = list_accounts(target.path()).expect("interrupted registry");
    assert_ne!(
        interrupted_registry.active_account_id,
        registry_before.active_account_id
    );
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("read installed phase"),
        Some(DeviceRestorePhase::Installed)
    );

    recover_interrupted_restore(target.path()).expect("recover registry commit cutpoint");
    assert_eq!(
        list_accounts(target.path()).expect("recovered registry"),
        registry_before
    );
    let resolved_db = ensure_accounts_initialized(target.path(), IdentityStorageMode::FileOnly)
        .expect("resolve recovered active account db path");
    assert_eq!(resolved_db, old_db);
    assert!(!restored_db.exists());
}

#[tokio::test]
async fn replacement_crash_after_journal_before_old_move_preserves_original() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (app_data, db_path) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir
        .path()
        .join("replacement-before-move.kukuri-backup");
    create_restore_fixture(app_data.path(), &db_path, &archive_path);
    let marker = db_path
        .parent()
        .expect("account directory")
        .join("original-after-backup.marker");
    fs::write(&marker, b"must survive").expect("write original marker");
    let before = app_data_snapshot(app_data.path());

    let prepared = prepare_restore_fixture(app_data.path(), &archive_path, true);
    let failure = fail_device_restore_at(DeviceRestoreTestFailurePoint::AfterInstallingJournal);
    let error = install_prepared_device_restore(app_data.path(), prepared)
        .err()
        .expect("simulated process stop must fail install");
    assert!(error.to_string().contains("simulated process stop"));
    drop(failure);

    assert_eq!(
        fs::read(&marker).expect("old marker before recovery"),
        b"must survive"
    );
    recover_interrupted_restore(app_data.path()).expect("recover journal-before-move stop");
    assert_eq!(app_data_snapshot(app_data.path()), before);
    recover_interrupted_restore(app_data.path()).expect("repeat completed recovery");
    assert_eq!(app_data_snapshot(app_data.path()), before);
}

#[tokio::test]
async fn replacement_rollback_can_resume_after_original_directory_was_restored() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (app_data, db_path) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir
        .path()
        .join("replacement-recovery-stop.kukuri-backup");
    create_restore_fixture(app_data.path(), &db_path, &archive_path);
    let marker = db_path
        .parent()
        .expect("account directory")
        .join("original-recovery.marker");
    fs::write(&marker, b"restore me once").expect("write original marker");
    let before = app_data_snapshot(app_data.path());

    let prepared = prepare_restore_fixture(app_data.path(), &archive_path, true);
    let install_stop =
        fail_device_restore_at(DeviceRestoreTestFailurePoint::AfterStagingDirectoryMove);
    install_prepared_device_restore(app_data.path(), prepared)
        .err()
        .expect("simulated installed-directory stop");
    drop(install_stop);

    let recovery_stop = fail_device_restore_at(
        DeviceRestoreTestFailurePoint::RollbackAfterOriginalDirectoryRestore,
    );
    let error = recover_interrupted_restore(app_data.path())
        .expect_err("simulated rollback stop must interrupt recovery");
    assert!(error.to_string().contains("simulated process stop"));
    drop(recovery_stop);
    assert_eq!(
        fs::read(&marker).expect("restored marker"),
        b"restore me once"
    );

    recover_interrupted_restore(app_data.path()).expect("resume interrupted rollback");
    assert_eq!(app_data_snapshot(app_data.path()), before);
}

#[tokio::test]
async fn new_account_install_stop_rolls_back_without_changing_existing_state() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("new-account-stop.kukuri-backup");
    create_device_backup(
        source.path(),
        &source_db,
        &CreateDeviceBackupRequest {
            path: archive_path.display().to_string(),
            passphrase: PASSPHRASE.to_string(),
            frontend_state: BTreeMap::from([("kukuri:draft".to_string(), "draft".to_string())]),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )
    .expect("create new-account rollback fixture");

    let (target, _target_db) = initialized_app_data().await;
    let before = app_data_snapshot(target.path());
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
    .expect("prepare new-account rollback fixture");
    let failure = fail_device_restore_at(DeviceRestoreTestFailurePoint::AfterInstalledJournal);
    install_prepared_device_restore(target.path(), prepared)
        .err()
        .expect("simulated new-account stop");
    drop(failure);
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("read installed phase"),
        Some(DeviceRestorePhase::Installed)
    );

    recover_interrupted_restore(target.path()).expect("rollback new account");
    assert_eq!(app_data_snapshot(target.path()), before);
    assert_eq!(
        pending_device_restore_frontend_state(target.path())
            .expect("rollback must not create frontend marker"),
        None
    );
}

#[tokio::test]
async fn replacement_stop_after_existing_directory_move_recovers_identity() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (app_data, db_path) = initialized_app_data().await;
    let identity_before = load_existing_keys(&db_path, IdentityStorageMode::FileOnly)
        .expect("load identity before restore")
        .expect("identity exists")
        .public_key_hex();
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("after-old-move.kukuri-backup");
    create_restore_fixture(app_data.path(), &db_path, &archive_path);
    let before = app_data_snapshot(app_data.path());

    let prepared = prepare_restore_fixture(app_data.path(), &archive_path, true);
    let failure = fail_device_restore_at(DeviceRestoreTestFailurePoint::AfterExistingDirectoryMove);
    install_prepared_device_restore(app_data.path(), prepared)
        .err()
        .expect("simulated stop after existing directory move");
    drop(failure);

    recover_interrupted_restore(app_data.path()).expect("recover moved existing directory");
    assert_eq!(app_data_snapshot(app_data.path()), before);
    let identity_after = load_existing_keys(&db_path, IdentityStorageMode::FileOnly)
        .expect("load identity after recovery")
        .expect("recovered identity exists")
        .public_key_hex();
    assert_eq!(identity_after, identity_before);
}

#[tokio::test]
async fn replacement_rollback_resumes_after_new_directory_was_removed() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (app_data, db_path) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir
        .path()
        .join("rollback-after-remove.kukuri-backup");
    create_restore_fixture(app_data.path(), &db_path, &archive_path);
    let before = app_data_snapshot(app_data.path());

    let prepared = prepare_restore_fixture(app_data.path(), &archive_path, true);
    let install_stop =
        fail_device_restore_at(DeviceRestoreTestFailurePoint::AfterStagingDirectoryMove);
    install_prepared_device_restore(app_data.path(), prepared)
        .err()
        .expect("simulated stop after restored directory move");
    drop(install_stop);

    let recovery_stop =
        fail_device_restore_at(DeviceRestoreTestFailurePoint::RollbackAfterFinalDirectoryRemoval);
    recover_interrupted_restore(app_data.path())
        .expect_err("simulated stop after restored directory removal");
    drop(recovery_stop);
    assert!(!db_path.exists());

    recover_interrupted_restore(app_data.path()).expect("resume rollback with missing final dir");
    assert_eq!(app_data_snapshot(app_data.path()), before);
}

#[tokio::test]
async fn activated_frontend_state_survives_finalize_restart_until_acknowledged() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("frontend-marker.kukuri-backup");
    let frontend_state = BTreeMap::from([
        ("kukuri:draft".to_string(), "draft body".to_string()),
        ("kukuri:workspace".to_string(), "main".to_string()),
    ]);
    create_device_backup(
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
    .expect("create frontend marker fixture");

    let (target, _target_db) = initialized_app_data().await;
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
    .expect("prepare frontend marker restore");
    assert_eq!(
        pending_device_restore_frontend_state(target.path()).expect("pre-install marker"),
        None
    );
    let installed =
        install_prepared_device_restore(target.path(), prepared).expect("install frontend restore");
    commit_device_restore(&installed).expect("commit frontend restore");
    mark_device_restore_awaiting_consent(target.path()).expect("mark awaiting consent");
    assert_eq!(
        pending_device_restore_frontend_state(target.path()).expect("pre-activation marker"),
        None
    );
    mark_device_restore_activated(target.path()).expect("mark activated");

    let failure = fail_device_restore_at(DeviceRestoreTestFailurePoint::AfterFrontendStateMarker);
    recover_interrupted_restore(target.path())
        .expect_err("simulated stop after frontend marker persistence");
    drop(failure);
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("activated journal remains"),
        Some(DeviceRestorePhase::Activated)
    );
    assert_eq!(
        pending_device_restore_frontend_state(target.path()).expect("durable frontend marker"),
        Some(frontend_state.clone())
    );
    let marker: serde_json::Value = serde_json::from_slice(
        &fs::read(target.path().join("device-restore-frontend-state.json"))
            .expect("read durable frontend marker"),
    )
    .expect("decode durable frontend marker");
    assert_eq!(marker["version"], 1);

    recover_interrupted_restore(target.path()).expect("finish activated cleanup after restart");
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("journal finalized"),
        None
    );
    assert_eq!(
        pending_device_restore_frontend_state(target.path()).expect("marker survives cleanup"),
        Some(frontend_state.clone())
    );
    let blocked = prepare_device_restore(
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
    .err()
    .expect("unacknowledged frontend marker must block another restore");
    assert!(blocked.to_string().contains("must be acknowledged"));
    acknowledge_pending_device_restore_frontend_state(target.path())
        .expect("acknowledge frontend marker");
    acknowledge_pending_device_restore_frontend_state(target.path())
        .expect("repeat frontend marker acknowledgement");
    assert_eq!(
        pending_device_restore_frontend_state(target.path()).expect("marker acknowledged"),
        None
    );
}

#[tokio::test]
async fn validation_only_checks_all_restored_inputs_without_creating_runtime_artifacts() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (source, source_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let archive_path = archive_dir.path().join("validation-only.kukuri-backup");
    create_restore_fixture(source.path(), &source_db, &archive_path);
    let (target, _target_db) = initialized_app_data().await;
    let prepared = prepare_restore_fixture(target.path(), &archive_path, false);
    let staging_db = prepared.staging_db_path();
    let iroh_root = staging_db.with_extension("iroh-data");
    let registry_before =
        fs::read(target.path().join("accounts.json")).expect("read registry before validation");

    validate_prepared_device_restore(&prepared)
        .await
        .expect("validate restored inputs without runtime");
    assert!(!iroh_root.exists());

    let identity_path = staging_db.with_extension("identity-key");
    let identity_bytes = fs::read(&identity_path).expect("read staged identity");
    fs::write(&identity_path, b"invalid identity").expect("corrupt staged identity");
    assert!(validate_prepared_device_restore(&prepared).await.is_err());
    fs::write(&identity_path, identity_bytes).expect("restore staged identity");

    let community_config = staging_db.with_extension("community-node.json");
    fs::write(
        &community_config,
        br#"{"nodes":[{"base_url":"not a URL"}]}"#,
    )
    .expect("write invalid community URL");
    assert!(validate_prepared_device_restore(&prepared).await.is_err());
    fs::remove_file(&community_config).expect("remove corrupt community config");

    let discovery_config = staging_db.with_extension("discovery.json");
    fs::write(&discovery_config, b"{").expect("corrupt discovery config");
    assert!(validate_prepared_device_restore(&prepared).await.is_err());
    fs::remove_file(&discovery_config).expect("remove corrupt discovery config");

    persist_optional_secret(
        &staging_db,
        IdentityStorageMode::FileOnly,
        PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
        PRIVATE_CHANNEL_CAPABILITIES_KEY,
        r#"[{"topic_id":"topic","channel_id":"channel","label":"label","creator_pubkey":"creator","namespace_secret_hex":"abcd"}]"#,
    )
    .expect("persist corrupt capability state");
    assert!(validate_prepared_device_restore(&prepared).await.is_err());
    persist_optional_secret(
        &staging_db,
        IdentityStorageMode::FileOnly,
        PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
        PRIVATE_CHANNEL_CAPABILITIES_KEY,
        "[]",
    )
    .expect("restore capability state");

    persist_optional_secret(
        &staging_db,
        IdentityStorageMode::FileOnly,
        GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
        GOSSIP_SUBSCRIPTION_STATE_KEY,
        "{",
    )
    .expect("persist corrupt gossip state");
    assert!(validate_prepared_device_restore(&prepared).await.is_err());
    persist_optional_secret(
        &staging_db,
        IdentityStorageMode::FileOnly,
        GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
        GOSSIP_SUBSCRIPTION_STATE_KEY,
        "{}",
    )
    .expect("restore gossip state");

    let database_bytes = fs::read(&staging_db).expect("read staged database");
    fs::write(&staging_db, b"not a sqlite database").expect("corrupt staged database");
    assert!(validate_prepared_device_restore(&prepared).await.is_err());
    fs::write(&staging_db, database_bytes).expect("restore staged database");

    assert!(!iroh_root.exists());
    assert_eq!(
        fs::read(target.path().join("accounts.json")).expect("read registry after validation"),
        registry_before
    );
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("validation must not create journal"),
        None
    );
}

#[tokio::test]
async fn post_journal_move_error_and_installed_journal_error_rollback_immediately() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;

    let (replacement, replacement_db) = initialized_app_data().await;
    let archive_dir = tempdir().expect("archive tempdir");
    let replacement_archive = archive_dir.path().join("move-error.kukuri-backup");
    create_restore_fixture(replacement.path(), &replacement_db, &replacement_archive);
    let replacement_before = app_data_snapshot(replacement.path());
    let prepared = prepare_restore_fixture(replacement.path(), &replacement_archive, true);
    let failure = fail_device_restore_at(DeviceRestoreTestFailurePoint::FailExistingDirectoryMove);
    install_prepared_device_restore(replacement.path(), prepared)
        .err()
        .expect("simulated existing directory move failure");
    drop(failure);
    assert_eq!(app_data_snapshot(replacement.path()), replacement_before);
    assert_eq!(
        pending_device_restore_phase(replacement.path()).expect("replacement pending phase"),
        None
    );

    let (source, source_db) = initialized_app_data().await;
    let new_archive = archive_dir
        .path()
        .join("installed-journal-error.kukuri-backup");
    create_restore_fixture(source.path(), &source_db, &new_archive);
    let (target, _target_db) = initialized_app_data().await;
    let target_before = app_data_snapshot(target.path());
    let prepared = prepare_restore_fixture(target.path(), &new_archive, false);
    let failure = fail_device_restore_at(DeviceRestoreTestFailurePoint::FailInstalledJournalWrite);
    install_prepared_device_restore(target.path(), prepared)
        .err()
        .expect("simulated installed journal write failure");
    drop(failure);
    assert_eq!(app_data_snapshot(target.path()), target_before);
    assert_eq!(
        pending_device_restore_phase(target.path()).expect("new-account pending phase"),
        None
    );
}

#[tokio::test]
async fn recovery_removes_only_exact_restore_staging_prefix_orphans() {
    let _resource = lock_test_resource(TestResource::IdentityStorage).await;
    let (app_data, _db_path) = initialized_app_data().await;
    let accounts = app_data.path().join("accounts");
    let orphan = accounts.join(".device-restore-staging-orphan");
    let near_prefix = accounts.join(".device-restore-stagingish-keep");
    fs::create_dir_all(&orphan).expect("create orphan staging");
    fs::create_dir_all(&near_prefix).expect("create near-prefix directory");

    recover_interrupted_restore(app_data.path()).expect("clean orphan staging");

    assert!(!orphan.exists());
    assert!(near_prefix.is_dir());
}

#[test]
fn device_restore_cancellation_can_be_rechecked_at_the_install_boundary() {
    let cancellation = DeviceBackupCancellation::default();
    cancellation.check().expect("fresh cancellation token");
    cancellation.cancel();
    assert!(cancellation.check().is_err());
}
