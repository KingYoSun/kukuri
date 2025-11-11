use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_RELATIVE_PATH: &str = "kukuri/cli_bootstrap_nodes.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliBootstrapCache {
    pub nodes: Vec<String>,
    pub updated_at_ms: u64,
}

impl CliBootstrapCache {
    pub fn new(nodes: Vec<String>) -> Self {
        let updated_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|dur| dur.as_millis() as u64)
            .unwrap_or_default();

        Self {
            nodes,
            updated_at_ms,
        }
    }
}

pub fn default_export_path() -> Option<PathBuf> {
    dirs::data_dir().map(|base| base.join(DEFAULT_RELATIVE_PATH))
}

pub fn resolve_export_path(explicit: Option<String>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return Some(PathBuf::from(path));
    }
    if let Ok(env_path) = std::env::var("KUKURI_CLI_BOOTSTRAP_PATH") {
        return Some(PathBuf::from(env_path));
    }
    default_export_path()
}

pub fn write_cache(cache: CliBootstrapCache, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create CLI bootstrap directory at {parent:?}"))?;
    }

    let mut nodes = cache.nodes;
    nodes.retain(|entry| !entry.trim().is_empty());
    nodes.dedup();

    if nodes.is_empty() {
        anyhow::bail!("no bootstrap nodes to export");
    }

    let payload = CliBootstrapCache {
        nodes,
        updated_at_ms: cache.updated_at_ms,
    };

    let json = serde_json::to_string_pretty(&payload)
        .context("failed to serialize CLI bootstrap cache")?;

    fs::write(path, json)
        .with_context(|| format!("failed to write CLI bootstrap cache to {}", path.display()))?;
    Ok(())
}
