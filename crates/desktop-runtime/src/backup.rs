use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use kukuri_core::{DeviceBackupReader, KukuriKeys};
use serde::{Deserialize, Serialize};

use crate::accounts::{
    ACCOUNTS_DIR_NAME, ACCOUNTS_REGISTRY_FILE_NAME, AccountRecord, account_db_path,
    account_id_for_pubkey, list_accounts, register_restored_account,
};
use crate::community_node::{
    COMMUNITY_NODE_CONSENT_PURPOSE, COMMUNITY_NODE_INVITE_CODE_PURPOSE,
    COMMUNITY_NODE_TOKEN_PURPOSE, load_community_node_config_from_file,
};
use crate::identity::{
    IdentityStorageMode, delete_optional_secret, load_existing_keys, load_optional_secret,
    persist_keys, persist_optional_secret, write_private_file_atomically,
};
use crate::paths::DB_FILE_NAME;
use crate::runtime::{
    GOSSIP_SUBSCRIPTION_STATE_KEY, GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
    PRIVATE_CHANNEL_CAPABILITIES_KEY, PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
};

const SECRETS_ENTRY: &str = "secret-bundle.json";
const FRONTEND_STATE_ENTRY: &str = "frontend-state.json";
const FILE_ENTRY_PREFIX: &str = "file/";
const FRONTEND_STATE_MAX_BYTES: usize = 2 * 1024 * 1024;
const RESTORE_JOURNAL_FILE: &str = "device-restore-journal.json";

mod create;

pub use create::create_device_backup;
use create::validate_frontend_state;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateDeviceBackupRequest {
    pub path: String,
    pub passphrase: String,
    #[serde(default)]
    pub frontend_state: BTreeMap<String, String>,
}

impl std::fmt::Debug for CreateDeviceBackupRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateDeviceBackupRequest")
            .field("path", &self.path)
            .field("passphrase", &"<redacted>")
            .field("frontend_state_keys", &self.frontend_state.keys())
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct PreviewDeviceBackupRequest {
    pub path: String,
    pub passphrase: String,
}

impl std::fmt::Debug for PreviewDeviceBackupRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewDeviceBackupRequest")
            .field("path", &self.path)
            .field("passphrase", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RestoreDeviceBackupRequest {
    pub path: String,
    pub passphrase: String,
    #[serde(default)]
    pub replace_existing: bool,
    #[serde(default)]
    pub apply_frontend_state: bool,
}

impl std::fmt::Debug for RestoreDeviceBackupRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestoreDeviceBackupRequest")
            .field("path", &self.path)
            .field("passphrase", &"<redacted>")
            .field("replace_existing", &self.replace_existing)
            .field("apply_frontend_state", &self.apply_frontend_state)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeviceBackupSummary {
    pub path: String,
    pub public_key: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(optional_fields = nullable))]
pub struct DeviceBackupPreview {
    pub public_key: String,
    #[serde(default)]
    pub account_label: Option<String>,
    pub created_at: i64,
    pub app_version: String,
    pub content_bytes: u64,
    pub existing_account_id: Option<String>,
    pub included: Vec<String>,
    pub requires_reconsent: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeviceBackupRestoreResult {
    pub account: AccountRecord,
    #[serde(default)]
    pub frontend_state: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum DeviceBackupPhase {
    Scanning,
    Encrypting,
    Decrypting,
    Installing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeviceBackupProgress {
    pub phase: DeviceBackupPhase,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Default)]
pub struct DeviceBackupCancellation(Arc<AtomicBool>);

impl DeviceBackupCancellation {
    pub fn reset(&self) {
        self.0.store(false, Ordering::Release);
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn check(&self) -> Result<()> {
        if self.0.load(Ordering::Acquire) {
            bail!("device backup operation canceled");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SecretBundleV1 {
    version: u32,
    identity_secret_hex: String,
    private_channel_capabilities: Option<String>,
    gossip_subscription_state: Option<String>,
    community_node_invite_codes: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
struct OptionalSecretValue {
    purpose: String,
    key: String,
    value: String,
}

pub struct PreparedDeviceRestore {
    staging_dir: PathBuf,
    public_key: String,
    account_label: Option<String>,
    frontend_state: BTreeMap<String, String>,
    replace_existing: bool,
}

impl PreparedDeviceRestore {
    pub fn staging_db_path(&self) -> PathBuf {
        self.staging_dir.join(DB_FILE_NAME)
    }
}

impl Drop for PreparedDeviceRestore {
    fn drop(&mut self) {
        if self.staging_dir.exists() {
            let _ = fs::remove_dir_all(&self.staging_dir);
        }
    }
}

pub struct InstalledDeviceRestore {
    app_data_dir: PathBuf,
    public_key: String,
    account_label: Option<String>,
    frontend_state: BTreeMap<String, String>,
    final_dir: PathBuf,
}

impl InstalledDeviceRestore {
    pub fn db_path(&self) -> PathBuf {
        self.final_dir.join(DB_FILE_NAME)
    }

    pub fn frontend_state(&self) -> &BTreeMap<String, String> {
        &self.frontend_state
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RestoreJournal {
    version: u32,
    phase: RestoreJournalPhase,
    final_dir: PathBuf,
    staging_dir: PathBuf,
    rollback_dir: Option<PathBuf>,
    registry_before: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RestoreJournalPhase {
    Installing,
    Installed,
    Committed,
}

pub fn preview_device_backup(
    app_data_dir: &Path,
    request: &PreviewDeviceBackupRequest,
) -> Result<DeviceBackupPreview> {
    let source = PathBuf::from(request.path.trim());
    ensure_backup_path_outside_app_data(app_data_dir, &source)?;
    let file = File::open(&source).context("failed to open device backup")?;
    let archive = DeviceBackupReader::open(file, &request.passphrase)?;
    let manifest = archive.manifest();
    let content_bytes = manifest.entries.iter().try_fold(0u64, |total, entry| {
        total
            .checked_add(entry.bytes)
            .ok_or_else(|| anyhow!("device backup total size overflow"))
    })?;
    let existing_account_id = list_accounts(app_data_dir)?
        .accounts
        .into_iter()
        .find(|account| account.pubkey == manifest.public_key)
        .map(|account| account.id);
    Ok(DeviceBackupPreview {
        public_key: manifest.public_key.clone(),
        account_label: manifest.account_label.clone(),
        created_at: manifest.created_at,
        app_version: manifest.app_version.clone(),
        content_bytes,
        existing_account_id,
        included: manifest.included.clone(),
        requires_reconsent: manifest.requires_reconsent.clone(),
    })
}

pub fn prepare_device_restore<F>(
    app_data_dir: &Path,
    request: &RestoreDeviceBackupRequest,
    cancellation: &DeviceBackupCancellation,
    mut progress: F,
) -> Result<PreparedDeviceRestore>
where
    F: FnMut(DeviceBackupProgress),
{
    recover_interrupted_restore(app_data_dir)?;
    cancellation.check()?;
    let source = PathBuf::from(request.path.trim());
    ensure_backup_path_outside_app_data(app_data_dir, &source)?;
    let file = File::open(&source).context("failed to open device backup")?;
    let mut archive = DeviceBackupReader::open(file, &request.passphrase)?;
    let manifest = archive.manifest().clone();
    let total_bytes = manifest.entries.iter().try_fold(0u64, |total, entry| {
        total
            .checked_add(entry.bytes)
            .ok_or_else(|| anyhow!("device backup total size overflow"))
    })?;
    let snapshot = list_accounts(app_data_dir)?;
    let existing = snapshot
        .accounts
        .iter()
        .find(|account| account.pubkey == manifest.public_key);
    if existing.is_some() && !request.replace_existing {
        bail!(
            "device backup account already exists; explicit replacement confirmation is required"
        );
    }

    let staging_dir = unique_account_work_dir(app_data_dir, ".device-restore-staging")?;
    fs::create_dir_all(&staging_dir)
        .context("failed to create device restore staging directory")?;
    let extraction = (|| -> Result<(SecretBundleV1, BTreeMap<String, String>)> {
        let mut secret_bundle = None;
        let mut frontend_state = BTreeMap::new();
        let mut completed = 0u64;
        for entry in &manifest.entries {
            cancellation.check()?;
            match entry.name.as_str() {
                SECRETS_ENTRY => {
                    if entry.bytes > FRONTEND_STATE_MAX_BYTES as u64 {
                        bail!("device backup secret bundle exceeds supported size");
                    }
                    let mut bytes = Vec::with_capacity(entry.bytes as usize);
                    archive.read_entry(&mut bytes, |delta| {
                        cancellation.check()?;
                        completed += delta;
                        progress(DeviceBackupProgress {
                            phase: DeviceBackupPhase::Decrypting,
                            completed_bytes: completed,
                            total_bytes,
                        });
                        Ok(())
                    })?;
                    secret_bundle = Some(
                        serde_json::from_slice(&bytes)
                            .context("invalid device backup secret bundle")?,
                    );
                }
                FRONTEND_STATE_ENTRY => {
                    if entry.bytes > FRONTEND_STATE_MAX_BYTES as u64 {
                        bail!("device backup frontend state exceeds supported size");
                    }
                    let mut bytes = Vec::with_capacity(entry.bytes as usize);
                    archive.read_entry(&mut bytes, |delta| {
                        cancellation.check()?;
                        completed += delta;
                        progress(DeviceBackupProgress {
                            phase: DeviceBackupPhase::Decrypting,
                            completed_bytes: completed,
                            total_bytes,
                        });
                        Ok(())
                    })?;
                    frontend_state = serde_json::from_slice(&bytes)
                        .context("invalid device backup frontend state")?;
                    validate_frontend_state(&frontend_state)?;
                }
                name if name.starts_with(FILE_ENTRY_PREFIX) => {
                    let relative = safe_relative_path(&name[FILE_ENTRY_PREFIX.len()..])?;
                    let target = staging_dir.join(relative);
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent)
                            .context("failed to create device restore entry directory")?;
                    }
                    let output = OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&target)
                        .with_context(|| format!("failed to create `{}`", target.display()))?;
                    archive.read_entry(output, |delta| {
                        cancellation.check()?;
                        completed += delta;
                        progress(DeviceBackupProgress {
                            phase: DeviceBackupPhase::Decrypting,
                            completed_bytes: completed,
                            total_bytes,
                        });
                        Ok(())
                    })?;
                }
                _ => bail!("device backup contains an unsupported entry"),
            }
        }
        archive.finish()?;
        let secret_bundle = secret_bundle
            .ok_or_else(|| anyhow!("device backup does not contain its secret bundle"))?;
        Ok((secret_bundle, frontend_state))
    })();

    let (secret_bundle, frontend_state) = match extraction {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(error);
        }
    };
    let validation = (|| -> Result<()> {
        if secret_bundle.version != 1 {
            bail!("unsupported device backup secret bundle version");
        }
        let keys = KukuriKeys::parse(&secret_bundle.identity_secret_hex)
            .context("device backup identity is invalid")?;
        if keys.public_key_hex() != manifest.public_key {
            bail!("device backup identity does not match its manifest");
        }
        let staging_db = staging_dir.join(DB_FILE_NAME);
        if !staging_db.is_file() {
            bail!("device backup does not contain its SQLite database");
        }
        persist_keys(&staging_db, IdentityStorageMode::FileOnly, &keys)?;
        persist_restored_secrets(&staging_db, &secret_bundle)
    })();
    if let Err(error) = validation {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(error);
    }

    Ok(PreparedDeviceRestore {
        staging_dir,
        public_key: manifest.public_key,
        account_label: manifest.account_label,
        frontend_state: if request.apply_frontend_state {
            frontend_state
        } else {
            BTreeMap::new()
        },
        replace_existing: request.replace_existing,
    })
}

pub fn install_prepared_device_restore(
    app_data_dir: &Path,
    prepared: PreparedDeviceRestore,
) -> Result<InstalledDeviceRestore> {
    recover_interrupted_restore(app_data_dir)?;
    let id = account_id_for_pubkey(&prepared.public_key)?;
    let final_db = account_db_path(app_data_dir, &id);
    let final_dir = final_db
        .parent()
        .ok_or_else(|| anyhow!("invalid restored account path"))?
        .to_path_buf();
    let snapshot = list_accounts(app_data_dir)?;
    let existing = snapshot
        .accounts
        .iter()
        .find(|account| account.pubkey == prepared.public_key);
    if existing.is_some() && !prepared.replace_existing {
        bail!(
            "device backup account already exists; explicit replacement confirmation is required"
        );
    }
    if existing.is_none() && final_dir.exists() {
        bail!("restored account directory already exists outside the registry");
    }

    let registry_path = app_data_dir.join(ACCOUNTS_REGISTRY_FILE_NAME);
    let registry_before = fs::read_to_string(&registry_path)
        .context("failed to snapshot accounts registry before restore")?;
    let (rollback_dir, rollback_secrets) = if final_dir.exists() {
        let secrets = make_account_file_self_contained(&final_db)?;
        (
            Some(unique_account_work_dir(
                app_data_dir,
                ".device-restore-rollback",
            )?),
            secrets,
        )
    } else {
        (None, Vec::new())
    };
    let mut journal = RestoreJournal {
        version: 1,
        phase: RestoreJournalPhase::Installing,
        final_dir: final_dir.clone(),
        staging_dir: prepared.staging_dir.clone(),
        rollback_dir: rollback_dir.clone(),
        registry_before,
    };
    write_restore_journal(app_data_dir, &journal)?;

    if let Some(rollback_dir) = &rollback_dir {
        fs::rename(&final_dir, rollback_dir)
            .context("failed to stage existing account rollback")?;
        if let Err(error) = scrub_keyring_optional_secrets(&final_db, &rollback_secrets) {
            let _ = rollback_from_journal(app_data_dir, &journal);
            return Err(error).context("failed to clear previous account secrets before restore");
        }
    }
    if let Err(error) = fs::rename(&prepared.staging_dir, &final_dir) {
        let _ = rollback_from_journal(app_data_dir, &journal);
        return Err(error).context("failed to install restored account directory");
    }
    journal.phase = RestoreJournalPhase::Installed;
    write_restore_journal(app_data_dir, &journal)?;

    Ok(InstalledDeviceRestore {
        app_data_dir: app_data_dir.to_path_buf(),
        public_key: prepared.public_key.clone(),
        account_label: prepared.account_label.clone(),
        frontend_state: prepared.frontend_state.clone(),
        final_dir,
    })
}

pub fn commit_device_restore(
    installed: &InstalledDeviceRestore,
) -> Result<DeviceBackupRestoreResult> {
    let account = register_restored_account(
        &installed.app_data_dir,
        &installed.public_key,
        installed.account_label.clone(),
    )?;
    let mut journal = load_restore_journal(&installed.app_data_dir)?
        .ok_or_else(|| anyhow!("device restore journal is missing"))?;
    journal.phase = RestoreJournalPhase::Committed;
    write_restore_journal(&installed.app_data_dir, &journal)?;
    Ok(DeviceBackupRestoreResult {
        account,
        frontend_state: installed.frontend_state.clone(),
    })
}

pub fn finalize_device_restore(installed: InstalledDeviceRestore) -> Result<()> {
    let journal = load_restore_journal(&installed.app_data_dir)?
        .ok_or_else(|| anyhow!("device restore journal is missing"))?;
    if journal.phase != RestoreJournalPhase::Committed {
        bail!("device restore cannot be finalized before registry commit");
    }
    if let Some(rollback_dir) = journal.rollback_dir
        && rollback_dir.exists()
    {
        fs::remove_dir_all(&rollback_dir)
            .context("failed to remove completed device restore rollback")?;
    }
    let path = restore_journal_path(&installed.app_data_dir);
    if path.exists() {
        fs::remove_file(path).context("failed to remove completed device restore journal")?;
    }
    Ok(())
}

pub fn rollback_device_restore(installed: &InstalledDeviceRestore) -> Result<()> {
    let journal = load_restore_journal(&installed.app_data_dir)?
        .ok_or_else(|| anyhow!("device restore journal is missing"))?;
    rollback_from_journal(&installed.app_data_dir, &journal)
}

pub fn recover_interrupted_restore(app_data_dir: &Path) -> Result<()> {
    let Some(journal) = load_restore_journal(app_data_dir)? else {
        return Ok(());
    };
    validate_journal_paths(app_data_dir, &journal)?;
    if journal.phase == RestoreJournalPhase::Committed {
        if let Some(rollback_dir) = &journal.rollback_dir
            && rollback_dir.exists()
        {
            fs::remove_dir_all(rollback_dir)
                .context("failed to finalize interrupted device restore")?;
        }
        if journal.staging_dir.exists() {
            fs::remove_dir_all(&journal.staging_dir)
                .context("failed to remove stale device restore staging")?;
        }
        fs::remove_file(restore_journal_path(app_data_dir))
            .context("failed to clear device restore journal")?;
        return Ok(());
    }
    rollback_from_journal(app_data_dir, &journal)
}

fn rollback_from_journal(app_data_dir: &Path, journal: &RestoreJournal) -> Result<()> {
    validate_journal_paths(app_data_dir, journal)?;
    if journal.final_dir.exists() {
        fs::remove_dir_all(&journal.final_dir)
            .context("failed to remove incomplete restored account")?;
    }
    if let Some(rollback_dir) = &journal.rollback_dir
        && rollback_dir.exists()
    {
        fs::rename(rollback_dir, &journal.final_dir)
            .context("failed to restore previous account directory")?;
    }
    if journal.staging_dir.exists() {
        fs::remove_dir_all(&journal.staging_dir)
            .context("failed to remove device restore staging")?;
    }
    write_private_file_atomically(
        &app_data_dir.join(ACCOUNTS_REGISTRY_FILE_NAME),
        journal.registry_before.as_bytes(),
    )?;
    let journal_path = restore_journal_path(app_data_dir);
    if journal_path.exists() {
        fs::remove_file(journal_path).context("failed to clear device restore journal")?;
    }
    Ok(())
}

fn active_account_for_db(app_data_dir: &Path, db_path: &Path) -> Result<AccountRecord> {
    let snapshot = list_accounts(app_data_dir)?;
    let active = snapshot
        .accounts
        .into_iter()
        .find(|account| account.id == snapshot.active_account_id)
        .ok_or_else(|| anyhow!("active account is missing from registry"))?;
    if account_db_path(app_data_dir, &active.id) != db_path {
        bail!("active account database does not match the running runtime");
    }
    Ok(active)
}

fn ensure_backup_path_outside_app_data(app_data_dir: &Path, path: &Path) -> Result<()> {
    let canonical_app_data = app_data_dir
        .canonicalize()
        .context("failed to resolve app data directory")?;
    let canonical_candidate = if path.exists() {
        path.canonicalize()
            .context("failed to resolve device backup path")?
    } else {
        let parent = path
            .parent()
            .filter(|value| !value.as_os_str().is_empty())
            .ok_or_else(|| anyhow!("device backup path must have a parent directory"))?;
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("device backup path must name a file"))?;
        parent
            .canonicalize()
            .context("failed to resolve device backup parent directory")?
            .join(file_name)
    };
    if canonical_candidate.starts_with(&canonical_app_data) {
        bail!("device backup file must be stored outside the kukuri app data directory");
    }
    Ok(())
}

fn collect_secret_bundle(db_path: &Path) -> Result<SecretBundleV1> {
    let keys = load_existing_keys(db_path, IdentityStorageMode::from_env())?
        .ok_or_else(|| anyhow!("active account identity is unavailable"))?;
    let private_channel_capabilities = load_optional_secret(
        db_path,
        IdentityStorageMode::from_env(),
        PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
        PRIVATE_CHANNEL_CAPABILITIES_KEY,
    )?;
    let gossip_subscription_state = load_optional_secret(
        db_path,
        IdentityStorageMode::from_env(),
        GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
        GOSSIP_SUBSCRIPTION_STATE_KEY,
    )?;
    let mut community_node_invite_codes = BTreeMap::new();
    if let Some(config) = load_community_node_config_from_file(db_path)? {
        for node in config.nodes {
            if let Some(value) = load_optional_secret(
                db_path,
                IdentityStorageMode::from_env(),
                COMMUNITY_NODE_INVITE_CODE_PURPOSE,
                &node.base_url,
            )? {
                community_node_invite_codes.insert(node.base_url, value);
            }
        }
    }
    Ok(SecretBundleV1 {
        version: 1,
        identity_secret_hex: keys.export_secret_hex(),
        private_channel_capabilities,
        gossip_subscription_state,
        community_node_invite_codes,
    })
}

fn persist_restored_secrets(db_path: &Path, bundle: &SecretBundleV1) -> Result<()> {
    if let Some(value) = &bundle.private_channel_capabilities {
        persist_optional_secret(
            db_path,
            IdentityStorageMode::FileOnly,
            PRIVATE_CHANNEL_CAPABILITIES_PURPOSE,
            PRIVATE_CHANNEL_CAPABILITIES_KEY,
            value,
        )?;
    }
    if let Some(value) = &bundle.gossip_subscription_state {
        persist_optional_secret(
            db_path,
            IdentityStorageMode::FileOnly,
            GOSSIP_SUBSCRIPTION_STATE_PURPOSE,
            GOSSIP_SUBSCRIPTION_STATE_KEY,
            value,
        )?;
    }
    for (base_url, value) in &bundle.community_node_invite_codes {
        persist_optional_secret(
            db_path,
            IdentityStorageMode::FileOnly,
            COMMUNITY_NODE_INVITE_CODE_PURPOSE,
            base_url,
            value,
        )?;
    }
    Ok(())
}

fn known_optional_secrets(db_path: &Path) -> Result<Vec<OptionalSecretValue>> {
    let mode = IdentityStorageMode::from_env();
    let mut pairs = vec![
        (
            PRIVATE_CHANNEL_CAPABILITIES_PURPOSE.to_string(),
            PRIVATE_CHANNEL_CAPABILITIES_KEY.to_string(),
        ),
        (
            GOSSIP_SUBSCRIPTION_STATE_PURPOSE.to_string(),
            GOSSIP_SUBSCRIPTION_STATE_KEY.to_string(),
        ),
    ];
    if let Some(config) = load_community_node_config_from_file(db_path)? {
        for node in config.nodes {
            for purpose in [
                COMMUNITY_NODE_TOKEN_PURPOSE,
                COMMUNITY_NODE_INVITE_CODE_PURPOSE,
                COMMUNITY_NODE_CONSENT_PURPOSE,
            ] {
                pairs.push((purpose.to_string(), node.base_url.clone()));
            }
        }
    }
    let mut values = Vec::new();
    for (purpose, key) in pairs {
        if let Some(value) = load_optional_secret(db_path, mode, &purpose, &key)? {
            values.push(OptionalSecretValue {
                purpose,
                key,
                value,
            });
        }
    }
    Ok(values)
}

fn make_account_file_self_contained(db_path: &Path) -> Result<Vec<OptionalSecretValue>> {
    if let Some(keys) = load_existing_keys(db_path, IdentityStorageMode::from_env())? {
        persist_keys(db_path, IdentityStorageMode::FileOnly, &keys)?;
    }
    let secrets = known_optional_secrets(db_path)?;
    for secret in &secrets {
        persist_optional_secret(
            db_path,
            IdentityStorageMode::FileOnly,
            &secret.purpose,
            &secret.key,
            &secret.value,
        )?;
    }
    Ok(secrets)
}

fn scrub_keyring_optional_secrets(db_path: &Path, secrets: &[OptionalSecretValue]) -> Result<()> {
    for secret in secrets {
        delete_optional_secret(
            db_path,
            IdentityStorageMode::Auto,
            &secret.purpose,
            &secret.key,
        )?;
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> Result<PathBuf> {
    if value.is_empty() || value.contains('\\') {
        bail!("device backup contains an invalid entry path");
    }
    let path = Path::new(value);
    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => safe.push(value),
            _ => bail!("device backup contains an invalid entry path"),
        }
    }
    if safe.as_os_str().is_empty() {
        bail!("device backup contains an empty entry path");
    }
    Ok(safe)
}

fn unique_account_work_dir(app_data_dir: &Path, prefix: &str) -> Result<PathBuf> {
    let root = app_data_dir.join(ACCOUNTS_DIR_NAME);
    fs::create_dir_all(&root).context("failed to create accounts directory")?;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    for attempt in 0..100u32 {
        let path = root.join(format!("{prefix}-{seed}-{}-{attempt}", std::process::id()));
        if !path.exists() {
            return Ok(path);
        }
    }
    bail!("failed to allocate device restore work directory")
}

fn restore_journal_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(RESTORE_JOURNAL_FILE)
}

fn write_restore_journal(app_data_dir: &Path, journal: &RestoreJournal) -> Result<()> {
    validate_journal_paths(app_data_dir, journal)?;
    let bytes = serde_json::to_vec(journal).context("failed to encode device restore journal")?;
    write_private_file_atomically(&restore_journal_path(app_data_dir), &bytes)
}

fn load_restore_journal(app_data_dir: &Path) -> Result<Option<RestoreJournal>> {
    let path = restore_journal_path(app_data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).context("failed to read device restore journal")?;
    let journal: RestoreJournal =
        serde_json::from_slice(&bytes).context("invalid device restore journal")?;
    if journal.version != 1 {
        bail!("unsupported device restore journal version");
    }
    Ok(Some(journal))
}

fn validate_journal_paths(app_data_dir: &Path, journal: &RestoreJournal) -> Result<()> {
    let accounts_root = app_data_dir.join(ACCOUNTS_DIR_NAME);
    for path in [
        Some(&journal.final_dir),
        Some(&journal.staging_dir),
        journal.rollback_dir.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if path.parent() != Some(accounts_root.as_path()) {
            bail!("device restore journal contains a path outside the accounts directory");
        }
    }
    Ok(())
}
