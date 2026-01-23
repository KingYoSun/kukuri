use anyhow::{Context, Result};
use nostr_sdk::prelude::{Keys, SecretKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
struct NodeKeyFile {
    secret_key: String,
    public_key: String,
}

pub fn load_or_generate(path: impl AsRef<Path>) -> Result<Keys> {
    let path = path.as_ref();
    if path.exists() {
        read_keys(path)
    } else {
        generate_keys(path)
    }
}

pub fn generate_keys(path: impl AsRef<Path>) -> Result<Keys> {
    let keys = Keys::generate();
    write_keys(path, &keys)?;
    Ok(keys)
}

pub fn rotate_keys(path: impl AsRef<Path>) -> Result<Keys> {
    let keys = Keys::generate();
    write_keys(path, &keys)?;
    Ok(keys)
}

pub fn read_keys(path: impl AsRef<Path>) -> Result<Keys> {
    let contents = fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read node key file: {}",
            path.as_ref().display()
        )
    })?;
    let file: NodeKeyFile = serde_json::from_str(&contents).context("invalid node key file")?;
    let secret = SecretKey::from_hex(&file.secret_key).context("invalid node secret key")?;
    Ok(Keys::new(secret))
}

pub fn key_path_from_env(var_name: &str, default_path: &str) -> Result<PathBuf> {
    let value = std::env::var(var_name).unwrap_or_else(|_| default_path.to_string());
    let path = PathBuf::from(value);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create node key directory: {}", parent.display())
        })?;
    }
    Ok(path)
}

fn write_keys(path: impl AsRef<Path>, keys: &Keys) -> Result<()> {
    let file = NodeKeyFile {
        secret_key: keys.secret_key().display_secret().to_string(),
        public_key: keys.public_key().to_hex(),
    };
    let json = serde_json::to_string_pretty(&file)?;
    fs::write(&path, json).with_context(|| {
        format!(
            "failed to write node key file: {}",
            path.as_ref().display()
        )
    })?;
    Ok(())
}

pub fn public_key_hex(keys: &Keys) -> String {
    keys.public_key().to_hex()
}

pub fn secret_key_hex(keys: &Keys) -> Result<String> {
    Ok(keys.secret_key().display_secret().to_string())
}
