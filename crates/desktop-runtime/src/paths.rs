use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub(crate) const DB_FILE_NAME: &str = "kukuri.db";
pub(crate) const DISCOVERY_CONFIG_FILE_EXTENSION: &str = "discovery.json";
pub(crate) const COMMUNITY_NODE_CONFIG_FILE_EXTENSION: &str = "community-node.json";

pub fn resolve_db_path_from_env(base_app_data_dir: &Path) -> Result<PathBuf> {
    let mut app_data_dir = std::env::var("KUKURI_APP_DATA_DIR")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| base_app_data_dir.to_path_buf());

    if app_data_dir == base_app_data_dir
        && let Some(instance) = std::env::var("KUKURI_INSTANCE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    {
        app_data_dir = app_data_dir.join(instance);
    }

    fs::create_dir_all(&app_data_dir)
        .with_context(|| format!("failed to create app data dir `{}`", app_data_dir.display()))?;

    let db_path = app_data_dir.join(DB_FILE_NAME);
    Ok(db_path)
}
pub(crate) fn discovery_config_path(db_path: &Path) -> PathBuf {
    db_path.with_extension(DISCOVERY_CONFIG_FILE_EXTENSION)
}

pub(crate) fn community_node_config_path(db_path: &Path) -> PathBuf {
    db_path.with_extension(COMMUNITY_NODE_CONFIG_FILE_EXTENSION)
}
