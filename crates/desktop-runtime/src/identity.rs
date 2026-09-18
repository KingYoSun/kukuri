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
        for account in keyring_account_candidates(db_path) {
            let _ = keyring.delete_password(KEYRING_SERVICE, account.as_str());
        }
    }
    Ok(())
}

fn delete_identity_with_keyring(
    db_path: &Path,
    mode: IdentityStorageMode,
    keyring: &dyn KeyringStore,
) -> Result<()> {
    if mode == IdentityStorageMode::Auto {
        for account in keyring_account_candidates(db_path) {
            keyring.delete_password(KEYRING_SERVICE, account.as_str())?;
        }
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
        for account in optional_secret_account_candidates(db_path, purpose, key) {
            match keyring.get_password(KEYRING_SERVICE, account.as_str()) {
                Ok(Some(secret)) => return Ok(Some(secret)),
                Ok(None) => {}
                // Headless Linux environments can have no default keyring provider at all.
                // Such environments could only have persisted this optional value through
                // the file fallback, so continue there without weakening other keyring errors.
                Err(error) if is_missing_default_keyring(&error) => break,
                Err(error) => {
                    return Err(error).context("failed to read optional secret from keyring");
                }
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
        for account in optional_secret_account_candidates(db_path, purpose, key) {
            let _ = keyring.delete_password(KEYRING_SERVICE, account.as_str());
        }
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
    for account in optional_secret_account_candidates(db_path, purpose, key) {
        match keyring.delete_password(KEYRING_SERVICE, account.as_str()) {
            Ok(()) => {}
            Err(error) if is_missing_default_keyring(&error) => return Ok(()),
            Err(error) => {
                return Err(error).context("failed to delete optional secret from keyring");
            }
        }
    }
    Ok(())
}

fn load_secret_from_keyring(db_path: &Path, keyring: &dyn KeyringStore) -> Result<Option<String>> {
    for account in keyring_account_candidates(db_path) {
        if let Some(secret) = keyring
            .get_password(KEYRING_SERVICE, account.as_str())
            .context("failed to read secret from keyring")?
        {
            return Ok(Some(secret));
        }
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
    format!("db:{}", resolve_db_path(db_path).display())
}

/// 読込・削除で試す keyring account。先頭が書込先の正規 account で、以降は旧版が
/// DB ファイル不在時に書いた未正規化パスの account(存在しうる場合のみ)。
fn keyring_account_candidates(db_path: &Path) -> Vec<String> {
    account_candidates(
        keyring_account(db_path),
        format!("db:{}", db_path.display()),
    )
}

/// DB ファイルの有無で keyring account が変わらないよう、ファイルが無ければ親ディレクトリを
/// 正規化して結合する。旧実装は `canonicalize(db_path)` 失敗時に生のパスへ倒していたため、
/// Windows のクリーンインストールでは作成時 `C:\...` と再読込時 `\\?\C:\...` で account が
/// 食い違い、keyring の identity に到達できなくなっていた。
fn resolve_db_path(db_path: &Path) -> PathBuf {
    if let Ok(resolved) = std::fs::canonicalize(db_path) {
        return resolved;
    }
    if let (Some(parent), Some(file_name)) = (db_path.parent(), db_path.file_name())
        && !parent.as_os_str().is_empty()
        && let Ok(parent) = std::fs::canonicalize(parent)
    {
        return parent.join(file_name);
    }
    db_path.to_path_buf()
}

fn account_candidates(primary: String, legacy: String) -> Vec<String> {
    if primary == legacy {
        vec![primary]
    } else {
        vec![primary, legacy]
    }
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
    optional_secret_account_for(resolve_db_path(db_path).as_path(), purpose, key)
}

fn optional_secret_account_candidates(db_path: &Path, purpose: &str, key: &str) -> Vec<String> {
    account_candidates(
        optional_secret_account(db_path, purpose, key),
        optional_secret_account_for(db_path, purpose, key),
    )
}

fn optional_secret_account_for(path: &Path, purpose: &str, key: &str) -> String {
    format!(
        "db:{}:{}:{}",
        path.display(),
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
mod tests;
