use std::fs::OpenOptions;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use keyring::{Entry, Error as KeyringError};
use nostr_sdk::prelude::{Keys, ToBech32};

const KEYRING_SERVICE: &str = "org.kukuri.next";
const BACKEND_FILE: &str = "file";
const BACKEND_KEYRING: &str = "keyring";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdentityStorageMode {
    Auto,
    FileOnly,
}

impl IdentityStorageMode {
    pub(crate) fn from_env() -> Self {
        match std::env::var("KUKURI_NEXT_DISABLE_KEYRING") {
            Ok(value) if matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES") => {
                Self::FileOnly
            }
            _ => Self::Auto,
        }
    }
}

pub(crate) fn load_or_create_keys(db_path: &Path, mode: IdentityStorageMode) -> Result<Keys> {
    if let Some(backend) = load_backend_marker(db_path)? {
        return load_keys_with_backend(db_path, backend.as_str(), mode);
    }

    if mode == IdentityStorageMode::Auto {
        match load_secret_from_keyring(db_path) {
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
        && persist_secret_to_keyring(db_path, encoded.as_str()).is_ok()
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
) -> Result<Keys> {
    match backend {
        BACKEND_KEYRING => {
            if mode == IdentityStorageMode::FileOnly {
                return Err(anyhow!(
                    "persisted identity is stored in keyring, but keyring is disabled"
                ));
            }
            let secret = load_secret_from_keyring(db_path)?
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

fn load_secret_from_keyring(db_path: &Path) -> Result<Option<String>> {
    let entry = Entry::new(KEYRING_SERVICE, keyring_account(db_path).as_str())
        .context("failed to initialize keyring entry")?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(anyhow!(error)).context("failed to read secret from keyring"),
    }
}

fn persist_secret_to_keyring(db_path: &Path, secret: &str) -> Result<()> {
    let entry = Entry::new(KEYRING_SERVICE, keyring_account(db_path).as_str())
        .context("failed to initialize keyring entry")?;
    entry
        .set_password(secret)
        .map_err(|error| anyhow!(error))
        .context("failed to persist secret into keyring")
}

fn load_secret_from_file(db_path: &Path) -> Result<Option<String>> {
    let path = key_file_path(db_path);
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
    let path = key_file_path(db_path);
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options
        .open(&path)
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
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&path).with_context(|| {
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

fn keyring_account(db_path: &Path) -> String {
    let resolved = std::fs::canonicalize(db_path).unwrap_or_else(|_| db_path.to_path_buf());
    format!("db:{}", resolved.display())
}

fn key_file_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("nsec")
}

fn backend_marker_path(db_path: &Path) -> PathBuf {
    db_path.with_extension("identity-store")
}
