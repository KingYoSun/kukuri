use std::collections::BTreeMap;

use kukuri_desktop_runtime::{
    CreateDeviceBackupRequest, DeviceBackupCancellation, PreviewDeviceBackupRequest,
    RestoreDeviceBackupRequest, commit_device_restore, create_device_backup,
    ensure_accounts_initialized_from_env, finalize_device_restore, install_prepared_device_restore,
    list_accounts, prepare_device_restore, preview_device_backup,
};

use crate::*;

pub(crate) async fn run_device_backup_restore(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    unsafe { std::env::set_var("KUKURI_DISABLE_KEYRING", "1") };
    let run_dir = tempfile::Builder::new()
        .prefix("device-backup-")
        .tempdir_in(artifacts_dir)?;
    let source_dir = run_dir.path().join("backup-source");
    let target_dir = run_dir.path().join("backup-target");
    std::fs::create_dir_all(&source_dir)?;
    std::fs::create_dir_all(&target_dir)?;
    let mut steps = Vec::new();

    let started = Instant::now();
    let source_db = ensure_accounts_initialized_from_env(&source_dir)?;
    let source_store = SqliteStore::connect_file(&source_db).await?;
    source_store.close().await;
    let iroh_dir = source_db.with_extension("iroh-data");
    std::fs::create_dir_all(iroh_dir.join("blobs"))?;
    std::fs::write(
        iroh_dir.join("blobs").join("scenario.bin"),
        b"portable state",
    )?;
    std::fs::write(iroh_dir.join("endpoint-secret.json"), b"device-only")?;
    push_named_step(&mut steps, "source_ready", started);

    let started = Instant::now();
    let backup_path = run_dir.path().join("account.kukuri-backup");
    let frontend_state = BTreeMap::from([("kukuri.desktop.locale".to_string(), "ja".to_string())]);
    create_device_backup(
        &source_dir,
        &source_db,
        &CreateDeviceBackupRequest {
            path: backup_path.display().to_string(),
            passphrase: "harness backup passphrase".to_string(),
            frontend_state: frontend_state.clone(),
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )?;
    anyhow::ensure!(backup_path.is_file(), "one-file backup was not created");
    push_named_step(&mut steps, "encrypted_backup_created", started);

    let target_db = ensure_accounts_initialized_from_env(&target_dir)?;
    let target_store = SqliteStore::connect_file(&target_db).await?;
    target_store.close().await;
    let target_before = list_accounts(&target_dir)?;

    let started = Instant::now();
    let wrong_passphrase = prepare_device_restore(
        &target_dir,
        &RestoreDeviceBackupRequest {
            path: backup_path.display().to_string(),
            passphrase: "wrong passphrase".to_string(),
            replace_existing: false,
            apply_frontend_state: true,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    );
    anyhow::ensure!(wrong_passphrase.is_err(), "wrong passphrase was accepted");
    anyhow::ensure!(
        list_accounts(&target_dir)? == target_before,
        "failed restore changed the target registry"
    );
    push_named_step(&mut steps, "wrong_passphrase_preserved_target", started);

    let started = Instant::now();
    let preview = preview_device_backup(
        &target_dir,
        &PreviewDeviceBackupRequest {
            path: backup_path.display().to_string(),
            passphrase: "harness backup passphrase".to_string(),
        },
    )?;
    anyhow::ensure!(preview.existing_account_id.is_none());
    let prepared = prepare_device_restore(
        &target_dir,
        &RestoreDeviceBackupRequest {
            path: backup_path.display().to_string(),
            passphrase: "harness backup passphrase".to_string(),
            replace_existing: false,
            apply_frontend_state: true,
        },
        &DeviceBackupCancellation::default(),
        |_| {},
    )?;
    let staged_store = SqliteStore::connect_file(prepared.staging_db_path()).await?;
    staged_store.close().await;
    let installed = install_prepared_device_restore(&target_dir, prepared)?;
    let restored_db = installed.db_path();
    let result = commit_device_restore(&installed)?;
    finalize_device_restore(installed)?;
    anyhow::ensure!(result.frontend_state == frontend_state);
    anyhow::ensure!(
        std::fs::read(
            restored_db
                .with_extension("iroh-data")
                .join("blobs")
                .join("scenario.bin")
        )? == b"portable state"
    );
    anyhow::ensure!(
        !restored_db
            .with_extension("iroh-data")
            .join("endpoint-secret.json")
            .exists(),
        "device-bound endpoint secret was restored"
    );
    anyhow::ensure!(list_accounts(&target_dir)?.active_account_id == result.account.id);
    push_named_step(&mut steps, "restored_and_activated", started);

    let run_dir = run_dir.keep();
    let result = HarnessResult {
        status: HarnessStatus::Pass,
        scenario: scenario.name.clone(),
        steps,
        artifacts: vec![run_dir.join("account.kukuri-backup").display().to_string()],
        metrics_snapshot: None,
    };
    write_result_artifact(root, artifacts_dir, &result)?;
    Ok(result)
}
