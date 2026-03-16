use std::fs::OpenOptions;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use keyring::{Entry, Error as KeyringError};
use nostr_sdk::prelude::{Keys, ToBech32};

const KEYRING_SERVICE: &str = "org.kukuri.desktop";
const BACKEND_FILE: &str = "file";
const BACKEND_KEYRING: &str = "keyring";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdentityStorageMode {
    Auto,
    FileOnly,
}

impl IdentityStorageMode {
    pub(crate) fn from_env() -> Self {
        match std::env::var("KUKURI_DISABLE_KEYRING") {
            Ok(value) if matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES") => {
                Self::FileOnly
            }
            _ => Self::Auto,
        }
    }
}

pub(crate) fn load_or_create_keys(db_path: &Path, mode: IdentityStorageMode) -> Result<Keys> {
    load_or_create_keys_with_keyring(db_path, mode, &SystemKeyringStore)
}

pub(crate) fn load_optional_secret(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
) -> Result<Option<String>> {
    load_optional_secret_with_keyring(db_path, mode, purpose, key, &SystemKeyringStore)
}

pub(crate) fn persist_optional_secret(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
    secret: &str,
) -> Result<()> {
    persist_optional_secret_with_keyring(db_path, mode, purpose, key, secret, &SystemKeyringStore)
}

pub(crate) fn delete_optional_secret(db_path: &Path, purpose: &str, key: &str) -> Result<()> {
    delete_optional_secret_with_keyring(db_path, purpose, key, &SystemKeyringStore)
}

fn load_or_create_keys_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    keyring: &dyn KeyringStore,
) -> Result<Keys> {
    if let Some(backend) = load_backend_marker(db_path)? {
        return load_keys_with_backend(db_path, backend.as_str(), mode, keyring);
    }

    if mode == IdentityStorageMode::Auto {
        match load_secret_from_keyring(db_path, keyring) {
            Ok(Some(secret)) => {
                write_backend_marker(db_path, BACKEND_KEYRING)?;
                return parse_keys(secret.as_str());
            }
            Ok(None) => {}
            Err(_) => {}
        }
    }

    if let Some(secret) = load_secret_from_file(db_path)? {
        write_backend_marker(db_path, BACKEND_FILE)?;
        return parse_keys(secret.as_str());
    }

    let keys = Keys::generate();
    let encoded = keys
        .secret_key()
        .to_bech32()
        .context("failed to encode generated secret key")?;

    if mode == IdentityStorageMode::Auto
        && persist_secret_to_keyring(db_path, encoded.as_str(), keyring).is_ok()
    {
        write_backend_marker(db_path, BACKEND_KEYRING)?;
    } else {
        persist_secret_to_file(db_path, encoded.as_str())?;
        write_backend_marker(db_path, BACKEND_FILE)?;
    }

    Ok(keys)
}

fn load_keys_with_backend(
    db_path: &Path,
    backend: &str,
    mode: IdentityStorageMode,
    keyring: &dyn KeyringStore,
) -> Result<Keys> {
    match backend {
        BACKEND_KEYRING => {
            if mode == IdentityStorageMode::FileOnly {
                return Err(anyhow!(
                    "persisted identity is stored in keyring, but keyring is disabled"
                ));
            }
            let secret = load_secret_from_keyring(db_path, keyring)?
                .ok_or_else(|| anyhow!("persisted keyring identity is unavailable"))?;
            parse_keys(secret.as_str())
        }
        BACKEND_FILE => {
            let secret = load_secret_from_file(db_path)?
                .ok_or_else(|| anyhow!("persisted identity file is unavailable"))?;
            parse_keys(secret.as_str())
        }
        other => Err(anyhow!("unknown identity backend `{other}`")),
    }
}

fn parse_keys(secret: &str) -> Result<Keys> {
    Keys::parse(secret).context("failed to parse persisted secret key")
}

fn load_optional_secret_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
    keyring: &dyn KeyringStore,
) -> Result<Option<String>> {
    if mode == IdentityStorageMode::Auto
        && let Some(secret) = keyring
            .get_password(
                KEYRING_SERVICE,
                optional_secret_account(db_path, purpose, key).as_str(),
            )
            .context("failed to read optional secret from keyring")?
    {
        return Ok(Some(secret));
    }

    load_secret_from_file_path(optional_secret_file_path(db_path, purpose, key).as_path())
}

fn persist_optional_secret_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
    secret: &str,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    let account = optional_secret_account(db_path, purpose, key);
    if mode == IdentityStorageMode::Auto
        && keyring
            .set_password(KEYRING_SERVICE, account.as_str(), secret)
            .is_ok()
    {
        let _ = delete_file_if_exists(optional_secret_file_path(db_path, purpose, key).as_path());
        return Ok(());
    }

    persist_secret_to_file_path(
        optional_secret_file_path(db_path, purpose, key).as_path(),
        secret,
    )
}

fn delete_optional_secret_with_keyring(
    db_path: &Path,
    purpose: &str,
    key: &str,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    let account = optional_secret_account(db_path, purpose, key);
    keyring.delete_password(KEYRING_SERVICE, account.as_str())?;
    delete_file_if_exists(optional_secret_file_path(db_path, purpose, key).as_path())?;
    Ok(())
}

fn load_secret_from_keyring(db_path: &Path, keyring: &dyn KeyringStore) -> Result<Option<String>> {
    let new_account = keyring_account(db_path);
    if let Some(secret) = keyring
        .get_password(KEYRING_SERVICE, new_account.as_str())
        .context("failed to read secret from keyring")?
    {
        return Ok(Some(secret));
    }

    let legacy_account = legacy_keyring_account(db_path);
    let Some(secret) = keyring
        .get_password(legacy_keyring_service().as_str(), legacy_account.as_str())
        .context("failed to read legacy secret from keyring")?
    else {
        return Ok(None);
    };

    keyring
        .set_password(KEYRING_SERVICE, new_account.as_str(), secret.as_str())
        .context("failed to migrate secret into new keyring service")?;
    keyring
        .delete_password(legacy_keyring_service().as_str(), legacy_account.as_str())
        .context("failed to delete legacy keyring secret")?;
    Ok(Some(secret))
}

fn persist_secret_to_keyring(
    db_path: &Path,
    secret: &str,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    keyring
        .set_password(KEYRING_SERVICE, keyring_account(db_path).as_str(), secret)
        .context("failed to persist secret into keyring")
}

fn load_secret_from_file(db_path: &Path) -> Result<Option<String>> {
    load_secret_from_file_path(key_file_path(db_path).as_path())
}

fn load_secret_from_file_path(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut secret = String::new();
    let mut file = std::fs::File::open(&path)
        .with_context(|| format!("failed to open identity file `{}`", path.display()))?;
    file.read_to_string(&mut secret)
        .with_context(|| format!("failed to read identity file `{}`", path.display()))?;
    Ok(Some(secret.trim().to_string()))
}

fn persist_secret_to_file(db_path: &Path, secret: &str) -> Result<()> {
    persist_secret_to_file_path(key_file_path(db_path).as_path(), secret)
}

fn persist_secret_to_file_path(path: &Path, secret: &str) -> Result<()> {
    let mut file = open_private_write_file(path)
        .with_context(|| format!("failed to create identity file `{}`", path.display()))?;
    file.write_all(secret.as_bytes())
        .with_context(|| format!("failed to write identity file `{}`", path.display()))?;
    Ok(())
}

fn load_backend_marker(db_path: &Path) -> Result<Option<String>> {
    let path = backend_marker_path(db_path);
    if !path.exists() {
        return Ok(None);
    }
    let mut backend = String::new();
    let mut file = std::fs::File::open(&path).with_context(|| {
        format!(
            "failed to open identity backend marker `{}`",
            path.display()
        )
    })?;
    file.read_to_string(&mut backend).with_context(|| {
        format!(
            "failed to read identity backend marker `{}`",
            path.display()
        )
    })?;
    Ok(Some(backend.trim().to_string()))
}

fn write_backend_marker(db_path: &Path, backend: &str) -> Result<()> {
    let path = backend_marker_path(db_path);
    let mut file = open_private_write_file(&path).with_context(|| {
        format!(
            "failed to create identity backend marker `{}`",
            path.display()
        )
    })?;
    file.write_all(backend.as_bytes()).with_context(|| {
        format!(
            "failed to write identity backend marker `{}`",
            path.display()
        )
    })?;
    Ok(())
}

fn open_private_write_file(path: &Path) -> Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    configure_private_file_options(&mut options);
    options
        .open(path)
        .with_context(|| format!("failed to open writable file `{}`", path.display()))
}

#[cfg(unix)]
fn configure_private_file_options(options: &mut OpenOptions) {
    options.mode(0o600);
}

#[cfg(not(unix))]
fn configure_private_file_options(_options: &mut OpenOptions) {}

fn keyring_account(db_path: &Path) -> String {
    let resolved = std::fs::canonicalize(db_path).unwrap_or_else(|_| db_path.to_path_buf());
    format!("db:{}", resolved.display())
}

fn legacy_keyring_account(db_path: &Path) -> String {
    let Some(parent) = db_path.parent() else {
        return keyring_account(db_path);
    };
    let legacy = parent.join(legacy_db_file_name());
    let resolved = std::fs::canonicalize(&legacy).unwrap_or(legacy);
    format!("db:{}", resolved.display())
}

fn legacy_db_file_name() -> String {
    format!("kukuri-{}.db", "next")
}

fn legacy_keyring_service() -> String {
    format!("org.kukuri.{}", "next")
}

fn key_file_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("nsec")
}

fn backend_marker_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("identity-store")
}

fn optional_secret_account(db_path: &Path, purpose: &str, key: &str) -> String {
    let resolved = std::fs::canonicalize(db_path).unwrap_or_else(|_| db_path.to_path_buf());
    format!(
        "db:{}:{}:{}",
        resolved.display(),
        purpose,
        optional_secret_suffix(key)
    )
}

fn optional_secret_file_path(db_path: &Path, purpose: &str, key: &str) -> PathBuf {
    db_path.with_extension(format!("{purpose}-{}", optional_secret_suffix(key)))
}

fn optional_secret_suffix(key: &str) -> String {
    blake3::hash(key.as_bytes()).to_hex().to_string()
}

fn delete_file_if_exists(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("failed to delete secret file `{}`", path.display()))?;
    }
    Ok(())
}

trait KeyringStore: Send + Sync {
    fn get_password(&self, service: &str, account: &str) -> Result<Option<String>>;
    fn set_password(&self, service: &str, account: &str, secret: &str) -> Result<()>;
    fn delete_password(&self, service: &str, account: &str) -> Result<()>;
}

struct SystemKeyringStore;

impl KeyringStore for SystemKeyringStore {
    fn get_password(&self, service: &str, account: &str) -> Result<Option<String>> {
        let entry = Entry::new(service, account).context("failed to initialize keyring entry")?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(anyhow!(error)).context("failed to read secret from keyring"),
        }
    }

    fn set_password(&self, service: &str, account: &str, secret: &str) -> Result<()> {
        let entry = Entry::new(service, account).context("failed to initialize keyring entry")?;
        entry
            .set_password(secret)
            .map_err(|error| anyhow!(error))
            .context("failed to persist secret into keyring")
    }

    fn delete_password(&self, service: &str, account: &str) -> Result<()> {
        let entry = Entry::new(service, account).context("failed to initialize keyring entry")?;
        match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(anyhow!(error)).context("failed to delete secret from keyring"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[derive(Clone, Default)]
    struct FakeKeyringStore {
        entries: Arc<Mutex<HashMap<(String, String), String>>>,
        fail_get: Arc<Mutex<bool>>,
        fail_set: Arc<Mutex<bool>>,
    }

    impl KeyringStore for FakeKeyringStore {
        fn get_password(&self, service: &str, account: &str) -> Result<Option<String>> {
            if *self.fail_get.lock().expect("keyring lock") {
                anyhow::bail!("fake keyring get failure");
            }
            Ok(self
                .entries
                .lock()
                .expect("keyring lock")
                .get(&(service.to_string(), account.to_string()))
                .cloned())
        }

        fn set_password(&self, service: &str, account: &str, secret: &str) -> Result<()> {
            if *self.fail_set.lock().expect("keyring lock") {
                anyhow::bail!("fake keyring set failure");
            }
            self.entries.lock().expect("keyring lock").insert(
                (service.to_string(), account.to_string()),
                secret.to_string(),
            );
            Ok(())
        }

        fn delete_password(&self, service: &str, account: &str) -> Result<()> {
            self.entries
                .lock()
                .expect("keyring lock")
                .remove(&(service.to_string(), account.to_string()));
            Ok(())
        }
    }

    fn clear_identity_env() {
        let legacy_disable_keyring = legacy_disable_keyring_env();
        for key in ["KUKURI_DISABLE_KEYRING", legacy_disable_keyring.as_str()] {
            unsafe { std::env::remove_var(key) };
        }
    }

    fn legacy_disable_keyring_env() -> String {
        format!("KUKURI_{}_DISABLE_KEYRING", "NEXT")
    }

    #[test]
    fn legacy_next_keyring_entry_migrates_to_kukuri_service() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let legacy_db_path = dir.path().join(legacy_db_file_name());
        let secret = Keys::generate().secret_key().to_bech32().expect("bech32");
        let keyring = FakeKeyringStore::default();
        keyring
            .set_password(
                legacy_keyring_service().as_str(),
                legacy_keyring_account(&legacy_db_path).as_str(),
                secret.as_str(),
            )
            .expect("seed legacy keyring");

        let keys = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
            .expect("migrated keys");

        assert_eq!(keys.secret_key().to_bech32().expect("bech32"), secret,);
        assert!(
            keyring
                .get_password(KEYRING_SERVICE, keyring_account(&db_path).as_str())
                .expect("new entry lookup")
                .is_some()
        );
        assert!(
            keyring
                .get_password(
                    legacy_keyring_service().as_str(),
                    legacy_keyring_account(&legacy_db_path).as_str()
                )
                .expect("legacy entry lookup")
                .is_none()
        );
    }

    #[test]
    fn auto_mode_prefers_keyring_secret_over_file_secret() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring_secret = Keys::generate().secret_key().to_bech32().expect("bech32");
        let file_secret = Keys::generate().secret_key().to_bech32().expect("bech32");
        let keyring = FakeKeyringStore::default();
        keyring
            .set_password(
                KEYRING_SERVICE,
                keyring_account(&db_path).as_str(),
                keyring_secret.as_str(),
            )
            .expect("seed keyring");
        persist_secret_to_file(&db_path, file_secret.as_str()).expect("seed file");

        let keys = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
            .expect("load keys");

        assert_eq!(
            keys.secret_key().to_bech32().expect("bech32"),
            keyring_secret
        );
        assert_eq!(
            load_backend_marker(&db_path).expect("load backend marker"),
            Some(BACKEND_KEYRING.to_string())
        );
    }

    #[test]
    fn auto_mode_falls_back_to_file_when_keyring_write_fails() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();
        *keyring.fail_set.lock().expect("keyring lock") = true;

        let keys = load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
            .expect("generate keys");

        assert_eq!(
            load_backend_marker(&db_path).expect("load backend marker"),
            Some(BACKEND_FILE.to_string())
        );
        assert_eq!(
            load_secret_from_file(&db_path).expect("load file secret"),
            Some(keys.secret_key().to_bech32().expect("bech32"))
        );
        assert!(
            keyring
                .get_password(KEYRING_SERVICE, keyring_account(&db_path).as_str())
                .expect("keyring lookup")
                .is_none()
        );
    }

    #[test]
    fn auto_mode_generated_keyring_secret_survives_restart() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();

        let original =
            load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
                .expect("create keys");
        let restarted =
            load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::Auto, &keyring)
                .expect("reload keys");

        assert_eq!(
            original.secret_key().to_bech32().expect("bech32"),
            restarted.secret_key().to_bech32().expect("bech32")
        );
        assert_eq!(
            load_backend_marker(&db_path).expect("load backend marker"),
            Some(BACKEND_KEYRING.to_string())
        );
    }

    #[test]
    fn file_only_mode_rejects_existing_keyring_backend_marker() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let secret = Keys::generate().secret_key().to_bech32().expect("bech32");
        let keyring = FakeKeyringStore::default();
        keyring
            .set_password(
                KEYRING_SERVICE,
                keyring_account(&db_path).as_str(),
                secret.as_str(),
            )
            .expect("seed keyring");
        write_backend_marker(&db_path, BACKEND_KEYRING).expect("write backend marker");

        let error =
            load_or_create_keys_with_keyring(&db_path, IdentityStorageMode::FileOnly, &keyring)
                .expect_err("file-only should reject keyring backend");

        assert!(
            error
                .to_string()
                .contains("persisted identity is stored in keyring"),
            "unexpected error: {error}"
        );
    }
}
