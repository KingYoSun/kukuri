use std::fs::OpenOptions;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use keyring::{Entry, Error as KeyringError};
use kukuri_core::KukuriKeys;

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

pub(crate) fn load_or_create_keys(db_path: &Path, mode: IdentityStorageMode) -> Result<KukuriKeys> {
    load_or_create_keys_with_keyring(db_path, mode, &SystemKeyringStore)
}

/// 既存の identity を「生成せずに」読み込む(accounts 移行の検出・再開用)。
/// backend marker があるのに実体へ到達できない場合は fail-loud で Err を返す。
pub(crate) fn load_existing_keys(
    db_path: &Path,
    mode: IdentityStorageMode,
) -> Result<Option<KukuriKeys>> {
    load_existing_keys_with_keyring(db_path, mode, &SystemKeyringStore)
}

/// 既知の鍵を db_path 配下の identity storage へ保存する(accounts 移行 / import 用)。
/// `load_or_create_keys` の新規生成分岐と同じ backend 選択(Auto: keyring 優先、
/// 失敗時 file)で永続化し、backend marker まで書き切る。
pub(crate) fn persist_keys(
    db_path: &Path,
    mode: IdentityStorageMode,
    keys: &KukuriKeys,
) -> Result<()> {
    persist_keys_with_keyring(db_path, mode, keys, &SystemKeyringStore)
}

/// db_path 配下の identity 実体(keyring entry / key file / legacy nsec / marker)を
/// すべて削除する(accounts 移行完了後の旧 flat レイアウト掃除用)。
pub(crate) fn delete_identity(db_path: &Path, mode: IdentityStorageMode) -> Result<()> {
    delete_identity_with_keyring(db_path, mode, &SystemKeyringStore)
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

pub(crate) fn delete_optional_secret(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
) -> Result<()> {
    delete_optional_secret_with_keyring(db_path, mode, purpose, key, &SystemKeyringStore)
}

fn load_or_create_keys_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    keyring: &dyn KeyringStore,
) -> Result<KukuriKeys> {
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

    let keys = KukuriKeys::generate();
    let encoded = keys.export_secret_hex();

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

pub(crate) fn load_existing_keys_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    keyring: &dyn KeyringStore,
) -> Result<Option<KukuriKeys>> {
    if let Some(backend) = load_backend_marker(db_path)? {
        return load_keys_with_backend(db_path, backend.as_str(), mode, keyring).map(Some);
    }
    if mode == IdentityStorageMode::Auto
        && let Ok(Some(secret)) = load_secret_from_keyring(db_path, keyring)
    {
        return parse_keys(secret.as_str()).map(Some);
    }
    if let Some(secret) = load_secret_from_file(db_path)? {
        return parse_keys(secret.as_str()).map(Some);
    }
    Ok(None)
}

pub(crate) fn persist_keys_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    keys: &KukuriKeys,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    let encoded = keys.export_secret_hex();
    if mode == IdentityStorageMode::Auto
        && persist_secret_to_keyring(db_path, encoded.as_str(), keyring).is_ok()
    {
        write_backend_marker(db_path, BACKEND_KEYRING)?;
        // 旧 file 実体が残ると marker=keyring と実体が食い違うため掃除する。
        let _ = delete_file_if_exists(key_file_path(db_path).as_path());
        let _ = delete_file_if_exists(legacy_key_file_path(db_path).as_path());
        return Ok(());
    }
    persist_secret_to_file(db_path, encoded.as_str())?;
    write_backend_marker(db_path, BACKEND_FILE)?;
    if mode == IdentityStorageMode::Auto {
        let _ = keyring.delete_password(KEYRING_SERVICE, keyring_account(db_path).as_str());
    }
    Ok(())
}

fn delete_identity_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    if mode == IdentityStorageMode::Auto {
        keyring.delete_password(KEYRING_SERVICE, keyring_account(db_path).as_str())?;
    }
    delete_file_if_exists(key_file_path(db_path).as_path())?;
    delete_file_if_exists(legacy_key_file_path(db_path).as_path())?;
    delete_file_if_exists(backend_marker_path(db_path).as_path())?;
    Ok(())
}

fn load_keys_with_backend(
    db_path: &Path,
    backend: &str,
    mode: IdentityStorageMode,
    keyring: &dyn KeyringStore,
) -> Result<KukuriKeys> {
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

fn parse_keys(secret: &str) -> Result<KukuriKeys> {
    KukuriKeys::parse(secret).context("failed to parse persisted secret key")
}

pub(crate) fn load_optional_secret_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
    keyring: &dyn KeyringStore,
) -> Result<Option<String>> {
    if mode == IdentityStorageMode::Auto {
        match keyring.get_password(
            KEYRING_SERVICE,
            optional_secret_account(db_path, purpose, key).as_str(),
        ) {
            Ok(Some(secret)) => return Ok(Some(secret)),
            Ok(None) => {}
            // Headless Linux environments can have no default keyring provider at all.
            // Such environments could only have persisted this optional value through
            // the file fallback, so continue there without weakening other keyring errors.
            Err(error) if is_missing_default_keyring(&error) => {}
            Err(error) => {
                return Err(error).context("failed to read optional secret from keyring");
            }
        }
    }

    load_secret_from_file_path(optional_secret_file_path(db_path, purpose, key).as_path())
}

fn is_missing_default_keyring(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<KeyringError>(),
            Some(KeyringError::NoDefaultStore)
        )
    })
}

pub(crate) fn persist_optional_secret_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
    secret: &str,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    let account = optional_secret_account(db_path, purpose, key);
    if mode == IdentityStorageMode::Auto {
        if keyring
            .set_password(KEYRING_SERVICE, account.as_str(), secret)
            .is_ok()
        {
            let _ =
                delete_file_if_exists(optional_secret_file_path(db_path, purpose, key).as_path());
            return Ok(());
        }
        // set 失敗時に旧 entry を残すと、load が keyring を優先するため file へ書いた
        // 新しい値が恒久的にシャドウされる(例: Windows Credential Manager の blob 上限
        // 超過で set が失敗し始めるケース)。best effort で削除してから file へ倒す。
        let _ = keyring.delete_password(KEYRING_SERVICE, account.as_str());
    }

    persist_secret_to_file_path(
        optional_secret_file_path(db_path, purpose, key).as_path(),
        secret,
    )
}

fn delete_optional_secret_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    purpose: &str,
    key: &str,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    if mode == IdentityStorageMode::Auto {
        delete_optional_secret_keyring_entry_with_keyring(db_path, purpose, key, keyring)?;
    }
    delete_file_if_exists(optional_secret_file_path(db_path, purpose, key).as_path())?;
    Ok(())
}

pub(crate) fn delete_optional_secret_keyring_entry_with_keyring(
    db_path: &Path,
    purpose: &str,
    key: &str,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    let account = optional_secret_account(db_path, purpose, key);
    match keyring.delete_password(KEYRING_SERVICE, account.as_str()) {
        Ok(()) => Ok(()),
        Err(error) if is_missing_default_keyring(&error) => Ok(()),
        Err(error) => Err(error).context("failed to delete optional secret from keyring"),
    }
}

fn load_secret_from_keyring(db_path: &Path, keyring: &dyn KeyringStore) -> Result<Option<String>> {
    if let Some(secret) = keyring
        .get_password(KEYRING_SERVICE, keyring_account(db_path).as_str())
        .context("failed to read secret from keyring")?
    {
        return Ok(Some(secret));
    }
    Ok(None)
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
    let primary = key_file_path(db_path);
    if let Some(secret) = load_secret_from_file_path(primary.as_path())? {
        return Ok(Some(secret));
    }
    load_secret_from_file_path(legacy_key_file_path(db_path).as_path())
}

fn load_secret_from_file_path(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut secret = String::new();
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("failed to open identity file `{}`", path.display()))?;
    file.read_to_string(&mut secret)
        .with_context(|| format!("failed to read identity file `{}`", path.display()))?;
    Ok(Some(secret.trim().to_string()))
}

fn persist_secret_to_file(db_path: &Path, secret: &str) -> Result<()> {
    persist_secret_to_file_path(key_file_path(db_path).as_path(), secret)?;
    delete_file_if_exists(legacy_key_file_path(db_path).as_path())
}

fn persist_secret_to_file_path(path: &Path, secret: &str) -> Result<()> {
    write_private_file_atomically(path, secret.as_bytes())
        .with_context(|| format!("failed to persist identity file `{}`", path.display()))
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
    write_private_file_atomically(&path, backend.as_bytes()).with_context(|| {
        format!(
            "failed to persist identity backend marker `{}`",
            path.display()
        )
    })
}

// 途中クラッシュで既存内容が破損しないよう、同一ディレクトリの temp ファイルへ
// write → fsync → rename で置換する(issue #574)。失敗は fail-loud で伝播させる。
pub(crate) fn write_private_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("invalid private file path `{}`", path.display()))?;
    let temp_path = path.with_file_name(format!("{file_name}.tmp"));
    let mut file = open_private_write_file(&temp_path)
        .with_context(|| format!("failed to create temp file `{}`", temp_path.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("failed to write temp file `{}`", temp_path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync temp file `{}`", temp_path.display()))?;
    drop(file);
    std::fs::rename(&temp_path, path).with_context(|| {
        format!(
            "failed to rename temp file `{}` to `{}`",
            temp_path.display(),
            path.display()
        )
    })?;
    sync_parent_dir(path)
}

// rename 自体の durability を確保するため、unix では親ディレクトリも fsync する。
// Windows は std にディレクトリ fsync の手段がないため rename の atomic 置換のみ。
#[cfg(unix)]
fn sync_parent_dir(path: &Path) -> Result<()> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => return Ok(()),
    };
    let dir = std::fs::File::open(parent)
        .with_context(|| format!("failed to open directory `{}`", parent.display()))?;
    dir.sync_all()
        .with_context(|| format!("failed to sync directory `{}`", parent.display()))
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) -> Result<()> {
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

fn key_file_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("identity-key")
}

// 互換パス(REFACTORING.md「互換パスと sunset 条件」参照)。
// 旧 `.nsec`(bech32 表記)は読み込んでも新形式 `.identity-key` へ再保存されず残り続ける。
// 鍵が見つからない場合 `load_or_create_keys` は黙って新しい鍵を生成するため、この読込パス
// だけを消すと旧ファイルの利用者が気づかないまま別人の鍵になる。
// 撤去条件(WP-C8 で確定): `.nsec` を検知したら起動を止めて案内を出す処理(fail-loud)と
// セットで撤去すること。単独では削除しない。
fn legacy_key_file_path(db_path: &Path) -> PathBuf {
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

pub(crate) trait KeyringStore: Send + Sync {
    fn get_password(&self, service: &str, account: &str) -> Result<Option<String>>;
    fn set_password(&self, service: &str, account: &str, secret: &str) -> Result<()>;
    fn delete_password(&self, service: &str, account: &str) -> Result<()>;
}

pub(crate) struct SystemKeyringStore;

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
        no_default_store: Arc<Mutex<bool>>,
        fail_set: Arc<Mutex<bool>>,
        fail_delete: Arc<Mutex<bool>>,
    }

    impl KeyringStore for FakeKeyringStore {
        fn get_password(&self, service: &str, account: &str) -> Result<Option<String>> {
            if *self.no_default_store.lock().expect("keyring lock") {
                return Err(anyhow!(KeyringError::NoDefaultStore));
            }
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
            if *self.no_default_store.lock().expect("keyring lock") {
                return Err(anyhow!(KeyringError::NoDefaultStore));
            }
            if *self.fail_delete.lock().expect("keyring lock") {
                anyhow::bail!("fake keyring delete failure");
            }
            self.entries
                .lock()
                .expect("keyring lock")
                .remove(&(service.to_string(), account.to_string()));
            Ok(())
        }
    }

    fn clear_identity_env() {
        unsafe { std::env::remove_var("KUKURI_DISABLE_KEYRING") };
    }

    #[test]
    fn auto_mode_prefers_keyring_secret_over_file_secret() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring_secret = KukuriKeys::generate().export_secret_hex();
        let file_secret = KukuriKeys::generate().export_secret_hex();
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

        assert_eq!(keys.export_secret_hex(), keyring_secret);
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
            Some(keys.export_secret_hex())
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

        assert_eq!(original.export_secret_hex(), restarted.export_secret_hex());
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
        let secret = KukuriKeys::generate().export_secret_hex();
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

    #[test]
    fn optional_secret_keyring_set_failure_does_not_shadow_file_fallback() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();

        persist_optional_secret_with_keyring(
            &db_path,
            IdentityStorageMode::Auto,
            "test-purpose",
            "registry",
            "old-value",
            &keyring,
        )
        .expect("persist to keyring");
        assert_eq!(
            load_optional_secret_with_keyring(
                &db_path,
                IdentityStorageMode::Auto,
                "test-purpose",
                "registry",
                &keyring,
            )
            .expect("load from keyring"),
            Some("old-value".to_string())
        );

        // keyring 書き込みが失敗し始めた後の更新(例: registry JSON が blob 上限を超過)。
        // 旧 entry が残ると load(keyring 優先)が file の新しい値を恒久的にシャドウする。
        *keyring.fail_set.lock().expect("keyring lock") = true;
        persist_optional_secret_with_keyring(
            &db_path,
            IdentityStorageMode::Auto,
            "test-purpose",
            "registry",
            "new-value",
            &keyring,
        )
        .expect("persist with keyring set failure");

        let loaded = load_optional_secret_with_keyring(
            &db_path,
            IdentityStorageMode::Auto,
            "test-purpose",
            "registry",
            &keyring,
        )
        .expect("load after fallback");
        assert_eq!(
            loaded,
            Some("new-value".to_string()),
            "stale keyring entry must not shadow the file fallback"
        );
    }

    #[test]
    fn optional_secret_uses_file_fallback_without_a_default_keyring() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();
        *keyring.no_default_store.lock().expect("keyring lock") = true;
        persist_secret_to_file_path(
            optional_secret_file_path(&db_path, "test-purpose", "registry").as_path(),
            "file-value",
        )
        .expect("seed file fallback");

        let loaded = load_optional_secret_with_keyring(
            &db_path,
            IdentityStorageMode::Auto,
            "test-purpose",
            "registry",
            &keyring,
        )
        .expect("missing default keyring must use file fallback");

        assert_eq!(loaded, Some("file-value".to_string()));
    }

    #[test]
    fn optional_secret_keyring_delete_treats_missing_default_store_as_absent() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();
        *keyring.no_default_store.lock().expect("keyring lock") = true;

        delete_optional_secret_keyring_entry_with_keyring(
            &db_path,
            "test-purpose",
            "registry",
            &keyring,
        )
        .expect("missing default keyring is equivalent to an absent entry");
    }

    #[test]
    fn file_only_optional_secret_ignores_keyring_shadow_and_failure() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();
        keyring
            .set_password(
                KEYRING_SERVICE,
                optional_secret_account(&db_path, "test-purpose", "registry").as_str(),
                "stale-keyring-value",
            )
            .expect("seed stale keyring value");
        persist_secret_to_file_path(
            optional_secret_file_path(&db_path, "test-purpose", "registry").as_path(),
            "staged-file-value",
        )
        .expect("seed staged file value");
        *keyring.fail_get.lock().expect("keyring lock") = true;

        let loaded = load_optional_secret_with_keyring(
            &db_path,
            IdentityStorageMode::FileOnly,
            "test-purpose",
            "registry",
            &keyring,
        )
        .expect("file-only load must not consult keyring");

        assert_eq!(loaded, Some("staged-file-value".to_string()));
    }

    #[test]
    fn optional_secret_persists_to_file_even_when_keyring_delete_fails() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keyring = FakeKeyringStore::default();

        *keyring.fail_set.lock().expect("keyring lock") = true;
        *keyring.fail_delete.lock().expect("keyring lock") = true;
        persist_optional_secret_with_keyring(
            &db_path,
            IdentityStorageMode::Auto,
            "test-purpose",
            "registry",
            "value",
            &keyring,
        )
        .expect("persist must fall back to file even when delete fails");

        assert_eq!(
            load_secret_from_file_path(
                optional_secret_file_path(&db_path, "test-purpose", "registry").as_path()
            )
            .expect("read fallback file"),
            Some("value".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn persist_secret_replaces_existing_file_via_rename() {
        use std::os::unix::fs::MetadataExt;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("kukuri.test-secret");
        persist_secret_to_file_path(&path, "old-value").expect("persist old value");
        let inode_before = std::fs::metadata(&path).expect("metadata before").ino();

        persist_secret_to_file_path(&path, "new-value").expect("persist new value");

        let inode_after = std::fs::metadata(&path).expect("metadata after").ino();
        assert_ne!(
            inode_before, inode_after,
            "persist must replace the file via rename, not truncate it in place"
        );
        assert_eq!(
            load_secret_from_file_path(&path).expect("load after persist"),
            Some("new-value".to_string())
        );
    }

    #[test]
    fn stale_temp_file_does_not_corrupt_persisted_secret() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("kukuri.test-secret");
        persist_secret_to_file_path(&path, "old-value").expect("persist old value");

        // 書き込み途中(rename 前)にクラッシュした状態を再現する。
        let temp_path = path.with_file_name("kukuri.test-secret.tmp");
        std::fs::write(&temp_path, "garbage-from-interrupted-write").expect("write stale temp");

        assert_eq!(
            load_secret_from_file_path(&path).expect("load with stale temp present"),
            Some("old-value".to_string()),
            "interrupted write must leave the previous content readable"
        );

        persist_secret_to_file_path(&path, "new-value").expect("persist over stale temp");
        assert_eq!(
            load_secret_from_file_path(&path).expect("load after recovery"),
            Some("new-value".to_string())
        );
        assert!(
            !temp_path.exists(),
            "persist must consume the temp file via rename"
        );
    }

    #[cfg(unix)]
    #[test]
    fn persisted_secret_file_keeps_private_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("kukuri.test-secret");
        persist_secret_to_file_path(&path, "old-value").expect("persist first value");
        persist_secret_to_file_path(&path, "new-value").expect("persist second value");

        let mode = std::fs::metadata(&path)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "secret file must stay private");
    }

    #[test]
    fn legacy_nsec_file_still_loads() {
        clear_identity_env();
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("kukuri.db");
        let keys = KukuriKeys::generate();
        let legacy_secret = kukuri_core::encode_secret_key_bech32(
            keys.export_secret_hex().as_str(),
            kukuri_core::LEGACY_SECRET_HRP,
        )
        .expect("legacy bech32");
        persist_secret_to_file_path(
            legacy_key_file_path(&db_path).as_path(),
            legacy_secret.as_str(),
        )
        .expect("persist legacy file");

        let restored = load_or_create_keys_with_keyring(
            &db_path,
            IdentityStorageMode::FileOnly,
            &FakeKeyringStore::default(),
        )
        .expect("load legacy keys");

        assert_eq!(restored.export_secret_hex(), keys.export_secret_hex());
    }
}
