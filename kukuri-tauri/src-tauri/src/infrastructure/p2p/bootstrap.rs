use crate::application::services::p2p_service::{P2PService, P2PStack};
use crate::domain::p2p::P2PEvent;
use crate::infrastructure::p2p::{bootstrap_config, metrics};
use crate::shared::config::AppConfig;
use anyhow::Context;
use base64::prelude::*;
use rand_core::{OsRng, TryRngCore};
use std::path::{Path, PathBuf};
use tauri::Manager;
use tokio::fs;
use tokio::sync::broadcast;

pub struct P2PBootstrapper {
    app_data_dir: PathBuf,
    config: AppConfig,
}

impl P2PBootstrapper {
    pub async fn new(app_handle: &tauri::AppHandle) -> anyhow::Result<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get app data dir: {}", e))?;

        tracing::info!("App data directory: {:?}", app_data_dir);

        if !app_data_dir.exists() {
            tracing::info!("Creating app data directory...");
            fs::create_dir_all(&app_data_dir)
                .await
                .with_context(|| format!("Failed to create app data dir at {app_data_dir:?}"))?;
            tracing::info!("App data directory created successfully");
        } else {
            tracing::info!("App data directory already exists");
        }

        let mut config = AppConfig::from_env();
        let selection = bootstrap_config::load_effective_bootstrap_nodes();
        config.network.bootstrap_peers = selection.nodes.clone();
        config.network.bootstrap_source = selection.source;
        tracing::info!(
            "Bootstrap peers source={:?} count={}",
            selection.source,
            selection.nodes.len()
        );

        if let Err(err) = config.validate() {
            return Err(anyhow::anyhow!(
                "Invalid application configuration: {}",
                err
            ));
        }

        Ok(Self {
            app_data_dir,
            config,
        })
    }

    pub fn app_data_dir(&self) -> &Path {
        &self.app_data_dir
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    fn node_key_path(&self) -> PathBuf {
        self.app_data_dir.join("p2p_node_secret.key")
    }

    pub async fn build_stack(
        &self,
        event_sender: broadcast::Sender<P2PEvent>,
    ) -> anyhow::Result<P2PStack> {
        metrics::reset_all();
        let secret_key = self.ensure_iroh_secret_key().await?;
        let builder = P2PService::builder(secret_key, self.config.network.clone())
            .with_event_sender(event_sender);

        builder
            .build()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to build P2P stack: {}", e))
    }

    async fn ensure_iroh_secret_key(&self) -> anyhow::Result<iroh::SecretKey> {
        let path = self.node_key_path();
        match fs::read_to_string(&path).await {
            Ok(contents) => {
                let trimmed = contents.trim();
                if trimmed.is_empty() {
                    tracing::warn!(
                        "Persisted iroh secret key at {:?} was empty; regenerating",
                        path
                    );
                    return self.generate_and_store_secret(&path).await;
                }

                let bytes = BASE64_STANDARD.decode(trimmed).map_err(|e| {
                    anyhow::anyhow!("Failed to decode persisted iroh secret key: {}", e)
                })?;
                let secret_bytes: [u8; 32] = bytes
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid iroh secret key length"))?;
                tracing::info!("Loaded persisted iroh secret key from {:?}", path);
                Ok(iroh::SecretKey::from_bytes(&secret_bytes))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                self.generate_and_store_secret(&path).await
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to read persisted iroh secret key at {:?}: {}. Regenerating.",
                    path,
                    err
                );
                self.generate_and_store_secret(&path).await
            }
        }
    }

    async fn generate_and_store_secret(&self, path: &Path) -> anyhow::Result<iroh::SecretKey> {
        let mut secret_bytes = [0u8; 32];
        OsRng
            .try_fill_bytes(&mut secret_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to generate iroh secret key: {:?}", e))?;
        let encoded = BASE64_STANDARD.encode(secret_bytes);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(path, encoded)
            .await
            .with_context(|| format!("Failed to write iroh secret key to {path:?}"))?;

        tracing::info!("Generated new iroh secret key at {:?}", path);
        Ok(iroh::SecretKey::from_bytes(&secret_bytes))
    }

    #[cfg(test)]
    pub fn from_parts(app_data_dir: PathBuf, config: AppConfig) -> Self {
        Self {
            app_data_dir,
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_bootstrapper(temp_dir: &TempDir) -> P2PBootstrapper {
        P2PBootstrapper::from_parts(temp_dir.path().to_path_buf(), AppConfig::default())
    }

    #[tokio::test]
    async fn generates_secret_if_missing() {
        let dir = TempDir::new().unwrap();
        let bootstrapper = test_bootstrapper(&dir);
        let secret = bootstrapper.ensure_iroh_secret_key().await.unwrap();

        let stored = tokio::fs::read_to_string(bootstrapper.node_key_path())
            .await
            .unwrap();
        let stored_bytes = BASE64_STANDARD.decode(stored.trim()).unwrap();
        let stored_secret =
            iroh::SecretKey::from_bytes(&stored_bytes.try_into().expect("invalid length"));

        assert_eq!(secret.to_bytes(), stored_secret.to_bytes());
    }

    #[tokio::test]
    async fn reuses_existing_secret() {
        let dir = TempDir::new().unwrap();
        let bootstrapper = test_bootstrapper(&dir);
        let first = bootstrapper.ensure_iroh_secret_key().await.unwrap();
        let second = bootstrapper.ensure_iroh_secret_key().await.unwrap();

        assert_eq!(first.to_bytes(), second.to_bytes());
    }
}
