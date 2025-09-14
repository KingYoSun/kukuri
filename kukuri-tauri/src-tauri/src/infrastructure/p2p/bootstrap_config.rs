/// ブートストラップノード設定モジュール
use crate::shared::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub development: EnvironmentConfig,
    pub staging: EnvironmentConfig,
    pub production: EnvironmentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub description: String,
    pub nodes: Vec<String>,
}

impl BootstrapConfig {
    /// 設定ファイルから読み込み
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let content = fs::read_to_string(path)
            .map_err(|e| AppError::ConfigurationError(format!("Failed to read bootstrap config: {}", e)))?;
        
        let config: BootstrapConfig = serde_json::from_str(&content)
            .map_err(|e| AppError::ConfigurationError(format!("Failed to parse bootstrap config: {}", e)))?;
        
        Ok(config)
    }
    
    /// デフォルト設定を取得
    pub fn default_config() -> Self {
        Self {
            development: EnvironmentConfig {
                description: "Local development bootstrap nodes".to_string(),
                nodes: vec![
                    "localhost:11223".to_string(),
                    "localhost:11224".to_string(),
                ],
            },
            staging: EnvironmentConfig {
                description: "Staging environment bootstrap nodes".to_string(),
                nodes: vec![],
            },
            production: EnvironmentConfig {
                description: "Production bootstrap nodes".to_string(),
                nodes: vec![],
            },
        }
    }
    
    /// 環境に応じたノードリストを取得
    pub fn get_nodes(&self, environment: &str) -> Vec<String> {
        match environment {
            "development" | "dev" => self.development.nodes.clone(),
            "staging" | "stage" => self.staging.nodes.clone(),
            "production" | "prod" => self.production.nodes.clone(),
            _ => {
                warn!("Unknown environment: {}, using development", environment);
                self.development.nodes.clone()
            }
        }
    }
    
    /// ソケットアドレスのリストを取得
    pub fn get_socket_addrs(&self, environment: &str) -> Vec<SocketAddr> {
        let nodes = self.get_nodes(environment);
        let mut addrs = Vec::new();
        
        for node in nodes {
            match node.parse::<SocketAddr>() {
                Ok(addr) => addrs.push(addr),
                Err(e) => {
                    debug!("Failed to parse address {}: {}", node, e);
                }
            }
        }
        
        addrs
    }
}

/// 現在の環境を取得
pub fn get_current_environment() -> String {
    std::env::var("KUKURI_ENV")
        .or_else(|_| std::env::var("ENVIRONMENT"))
        .unwrap_or_else(|_| "development".to_string())
}

/// ブートストラップノードを読み込み
pub fn load_bootstrap_nodes() -> Result<Vec<SocketAddr>, AppError> {
    let env = get_current_environment();
    info!("Loading bootstrap nodes for environment: {}", env);
    
    // まず設定ファイルを探す
    let config_path = "bootstrap_nodes.json";
    let config = if Path::new(config_path).exists() {
        BootstrapConfig::load_from_file(config_path)?
    } else {
        info!("Bootstrap config file not found, using defaults");
        BootstrapConfig::default_config()
    };
    
    let addrs = config.get_socket_addrs(&env);
    
    if addrs.is_empty() {
        warn!("No bootstrap nodes configured for environment: {}", env);
    } else {
        info!("Loaded {} bootstrap nodes", addrs.len());
    }
    
    Ok(addrs)
}
