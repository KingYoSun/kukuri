use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use kukuri_core::{
    DEVICE_BACKUP_COMPONENT_VERSION, DEVICE_BACKUP_FORMAT_VERSION, DEVICE_BACKUP_MAX_ENTRY_BYTES,
    DEVICE_BACKUP_MAX_TOTAL_BYTES, DeviceBackupEntryV1, DeviceBackupManifestV1, DeviceBackupWriter,
};

use super::{
    CreateDeviceBackupRequest, DeviceBackupCancellation, DeviceBackupOutputFile, DeviceBackupPhase,
    DeviceBackupProgress, DeviceBackupSummary, FILE_ENTRY_PREFIX, FRONTEND_STATE_ENTRY,
    FRONTEND_STATE_MAX_BYTES, SECRETS_ENTRY, active_account_for_db, collect_secret_bundle,
    ensure_backup_path_outside_app_data,
};
use crate::{IDEMPOTENCY_LEDGER_FILE_NAME, idempotency_ledger_path, paths::DB_FILE_NAME};

const IROH_ENDPOINT_SECRET_FILE: &str = "endpoint-secret.json";

enum EntrySource {
    File(PathBuf),
    Bytes(Vec<u8>),
}

struct SourceEntry {
    manifest: DeviceBackupEntryV1,
    source: EntrySource,
}

pub fn create_device_backup<F>(
    app_data_dir: &Path,
    db_path: &Path,
    request: &CreateDeviceBackupRequest,
    cancellation: &DeviceBackupCancellation,
    mut progress: F,
) -> Result<DeviceBackupSummary>
where
    F: FnMut(DeviceBackupProgress),
{
    cancellation.check()?;
    validate_frontend_state(&request.frontend_state)?;
    let destination = PathBuf::from(request.path.trim());
    if destination.as_os_str().is_empty() {
        bail!("device backup destination is required");
    }
    if destination.exists() {
        bail!("device backup destination already exists");
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| anyhow!("device backup destination must have a parent directory"))?;
    if !parent.exists() {
        bail!("device backup destination directory does not exist");
    }
    ensure_backup_path_outside_app_data(app_data_dir, &destination)?;

    let account = active_account_for_db(app_data_dir, db_path)?;
    let secret_bytes = serde_json::to_vec(&collect_secret_bundle(db_path)?)
        .context("failed to encode device backup secrets")?;
    let frontend_bytes = serde_json::to_vec(&request.frontend_state)
        .context("failed to encode device backup frontend state")?;
    let mut raw_sources = account_file_sources(db_path)?;
    raw_sources.push((SECRETS_ENTRY.to_string(), EntrySource::Bytes(secret_bytes)));
    raw_sources.push((
        FRONTEND_STATE_ENTRY.to_string(),
        EntrySource::Bytes(frontend_bytes),
    ));
    let includes_idempotency_ledger = raw_sources
        .iter()
        .any(|(name, _)| name == "file/kukuri.idempotency.sqlite3");
    let total_bytes = raw_sources.iter().try_fold(0u64, |total, (_, source)| {
        let bytes = source_len(source)?;
        let total = total
            .checked_add(bytes)
            .ok_or_else(|| anyhow!("device backup total size overflow"))?;
        if total > DEVICE_BACKUP_MAX_TOTAL_BYTES {
            bail!("device backup total size exceeds supported bounds");
        }
        Ok(total)
    })?;

    let mut scanned = 0u64;
    let mut sources = Vec::with_capacity(raw_sources.len());
    for (name, source) in raw_sources {
        cancellation.check()?;
        let (bytes, hash) = hash_source(&source, cancellation, |delta| {
            scanned += delta;
            progress(DeviceBackupProgress {
                phase: DeviceBackupPhase::Scanning,
                completed_bytes: scanned,
                total_bytes,
            });
        })?;
        sources.push(SourceEntry {
            manifest: DeviceBackupEntryV1 {
                name,
                bytes,
                blake3: hash,
            },
            source,
        });
    }

    let mut included = vec![
        "account_key".to_string(),
        "sqlite".to_string(),
        "local_docs_and_blobs".to_string(),
        "private_channel_state".to_string(),
        "drafts_and_preferences".to_string(),
        "community_node_configuration".to_string(),
        "desired_subscriptions".to_string(),
    ];
    if includes_idempotency_ledger {
        included.push("idempotency_ledger".to_string());
    }
    let manifest = DeviceBackupManifestV1 {
        format_version: DEVICE_BACKUP_FORMAT_VERSION,
        component_version: DEVICE_BACKUP_COMPONENT_VERSION,
        created_at: unix_seconds(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        public_key: account.pubkey.clone(),
        account_label: account.label.clone(),
        included,
        requires_reconsent: vec![
            "app_legal_documents".to_string(),
            "age_attestation".to_string(),
            "community_node_policies".to_string(),
            "adult_content_display".to_string(),
        ],
        entries: sources.iter().map(|entry| entry.manifest.clone()).collect(),
    };

    let temp_path = partial_path(&destination)?;
    let result = (|| -> Result<()> {
        let file = DeviceBackupOutputFile::create_new(&temp_path)
            .with_context(|| format!("failed to create `{}`", temp_path.display()))?;
        let mut archive = DeviceBackupWriter::new(file, &request.passphrase, manifest)?;
        let mut written = 0u64;
        for source in &sources {
            cancellation.check()?;
            match &source.source {
                EntrySource::File(path) => {
                    let file = File::open(path)
                        .with_context(|| format!("failed to read `{}`", path.display()))?;
                    archive.write_entry(file, |delta| {
                        cancellation.check()?;
                        written += delta;
                        progress(DeviceBackupProgress {
                            phase: DeviceBackupPhase::Encrypting,
                            completed_bytes: written,
                            total_bytes,
                        });
                        Ok(())
                    })?;
                }
                EntrySource::Bytes(bytes) => {
                    archive.write_entry(Cursor::new(bytes), |delta| {
                        cancellation.check()?;
                        written += delta;
                        progress(DeviceBackupProgress {
                            phase: DeviceBackupPhase::Encrypting,
                            completed_bytes: written,
                            total_bytes,
                        });
                        Ok(())
                    })?;
                }
            }
        }
        let file = archive.finish()?;
        file.sync_all().context("failed to sync device backup")?;
        drop(file);
        fs::rename(&temp_path, &destination).with_context(|| {
            format!(
                "failed to finalize device backup `{}`",
                destination.display()
            )
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result?;

    let bytes = fs::metadata(&destination)
        .context("failed to inspect completed device backup")?
        .len();
    Ok(DeviceBackupSummary {
        path: destination.display().to_string(),
        public_key: account.pubkey,
        bytes,
    })
}

fn account_file_sources(db_path: &Path) -> Result<Vec<(String, EntrySource)>> {
    let mut sources = Vec::new();
    add_file_source(&mut sources, db_path, Path::new(DB_FILE_NAME))?;
    for extension in ["db-wal", "db-shm"] {
        let path = db_path.with_extension(extension);
        if path.is_file() {
            let name = path
                .file_name()
                .ok_or_else(|| anyhow!("invalid SQLite companion filename"))?;
            add_file_source(&mut sources, &path, Path::new(name))?;
        }
    }
    for extension in [
        "discovery.json",
        "community-node.json",
        "subscriptions.json",
    ] {
        let path = db_path.with_extension(extension);
        if path.is_file() {
            let name = path
                .file_name()
                .ok_or_else(|| anyhow!("invalid account state filename"))?;
            add_file_source(&mut sources, &path, Path::new(name))?;
        }
    }
    let idempotency_path = idempotency_ledger_path(db_path);
    if idempotency_path.is_file() {
        add_file_source(
            &mut sources,
            &idempotency_path,
            Path::new(IDEMPOTENCY_LEDGER_FILE_NAME),
        )?;
    }
    let iroh_root = db_path.with_extension("iroh-data");
    if iroh_root.exists() {
        collect_tree_sources(&iroh_root, &iroh_root, &mut sources)?;
    }
    Ok(sources)
}

fn collect_tree_sources(
    root: &Path,
    current: &Path,
    sources: &mut Vec<(String, EntrySource)>,
) -> Result<()> {
    let mut entries = fs::read_dir(current)
        .with_context(|| format!("failed to read `{}`", current.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_type = entry
            .file_type()
            .context("failed to inspect account state entry")?;
        if file_type.is_symlink() {
            bail!("device backup does not follow symbolic links");
        }
        if file_type.is_dir() {
            collect_tree_sources(root, &entry.path(), sources)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = entry.path().strip_prefix(root)?.to_path_buf();
        if relative == Path::new(IROH_ENDPOINT_SECRET_FILE) {
            continue;
        }
        add_file_source(
            sources,
            &entry.path(),
            &Path::new("kukuri.iroh-data").join(relative),
        )?;
    }
    Ok(())
}

fn add_file_source(
    sources: &mut Vec<(String, EntrySource)>,
    path: &Path,
    relative: &Path,
) -> Result<()> {
    if !path.is_file() {
        bail!(
            "required device backup file `{}` is missing",
            path.display()
        );
    }
    let name = path_to_archive_name(relative)?;
    sources.push((
        format!("{FILE_ENTRY_PREFIX}{name}"),
        EntrySource::File(path.to_path_buf()),
    ));
    Ok(())
}

fn path_to_archive_name(path: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => parts.push(
                value
                    .to_str()
                    .ok_or_else(|| anyhow!("device backup path is not valid UTF-8"))?,
            ),
            _ => bail!("device backup path is not relative"),
        }
    }
    if parts.is_empty() {
        bail!("device backup path is empty");
    }
    Ok(parts.join("/"))
}

fn source_len(source: &EntrySource) -> Result<u64> {
    let bytes = match source {
        EntrySource::File(path) => fs::metadata(path)
            .with_context(|| format!("failed to inspect `{}`", path.display()))?
            .len(),
        EntrySource::Bytes(bytes) => bytes.len() as u64,
    };
    if bytes > DEVICE_BACKUP_MAX_ENTRY_BYTES {
        bail!("device backup entry exceeds supported size");
    }
    Ok(bytes)
}

fn hash_source<F>(
    source: &EntrySource,
    cancellation: &DeviceBackupCancellation,
    mut progress: F,
) -> Result<(u64, String)>
where
    F: FnMut(u64),
{
    let mut hasher = blake3::Hasher::new();
    let mut total = 0u64;
    match source {
        EntrySource::File(path) => {
            let mut file =
                File::open(path).with_context(|| format!("failed to read `{}`", path.display()))?;
            let mut buffer = vec![0u8; 64 * 1024];
            loop {
                cancellation.check()?;
                let read = file
                    .read(&mut buffer)
                    .context("failed to hash backup entry")?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
                total += read as u64;
                progress(read as u64);
            }
        }
        EntrySource::Bytes(bytes) => {
            cancellation.check()?;
            hasher.update(bytes);
            total = bytes.len() as u64;
            progress(total);
        }
    }
    Ok((total, hasher.finalize().to_hex().to_string()))
}

pub(super) fn validate_frontend_state(state: &BTreeMap<String, String>) -> Result<()> {
    if state.len() > 32 {
        bail!("device backup frontend state has too many keys");
    }
    let encoded = serde_json::to_vec(state).context("failed to validate frontend state")?;
    if encoded.len() > FRONTEND_STATE_MAX_BYTES {
        bail!("device backup frontend state exceeds supported size");
    }
    Ok(())
}

fn partial_path(destination: &Path) -> Result<PathBuf> {
    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("invalid device backup destination filename"))?;
    Ok(destination.with_file_name(format!(".{name}.partial")))
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
