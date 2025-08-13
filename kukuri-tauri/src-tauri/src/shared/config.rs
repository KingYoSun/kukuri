use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
    pub sync: SyncConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub bootstrap_peers: Vec<String>,
    pub max_peers: u32,
    pub connection_timeout: u64,
    pub retry_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub auto_sync: bool,
    pub sync_interval: u64,
    pub max_retry: u32,
    pub batch_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: String,
    pub cache_size: u64,
    pub cache_ttl: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                url: "sqlite:data/kukuri.db".to_string(),
                max_connections: 5,
                connection_timeout: 30,
            },
            network: NetworkConfig {
                bootstrap_peers: vec![],
                max_peers: 50,
                connection_timeout: 30,
                retry_interval: 60,
            },
            sync: SyncConfig {
                auto_sync: true,
                sync_interval: 300, // 5 minutes
                max_retry: 3,
                batch_size: 100,
            },
            storage: StorageConfig {
                data_dir: "./data".to_string(),
                cache_size: 100 * 1024 * 1024, // 100MB
                cache_ttl: 3600, // 1 hour
            },
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        // Load from environment variables or config file
        // For now, use defaults
        Self::default()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.database.max_connections == 0 {
            return Err("Database max_connections must be greater than 0".to_string());
        }
        if self.network.max_peers == 0 {
            return Err("Network max_peers must be greater than 0".to_string());
        }
        Ok(())
    }
}