/// ブートストラップノード設定モジュール
use crate::shared::error::AppError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use iroh::{NodeAddr, NodeId};
use std::str::FromStr;
use dirs;

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

    /// 形式: "<node_id>@<host:port>" のみ NodeAddr に変換する
    /// SocketAddr のみの指定は警告を出してスキップ
    pub fn get_node_addrs_with_id(&self, environment: &str) -> Vec<NodeAddr> {
        let nodes = self.get_nodes(environment);
        let mut out = Vec::new();

        for node in nodes {
            if let Some((id_part, addr_part)) = node.split_once('@') {
                match (NodeId::from_str(id_part), addr_part.parse::<SocketAddr>()) {
                    (Ok(node_id), Ok(sock)) => {
                        out.push(NodeAddr::new(node_id).with_direct_addresses([sock]));
                    }
                    (id_res, addr_res) => {
                        debug!(
                            "Invalid node entry '{}': id_ok={}, addr_ok={}",
                            node,
                            id_res.is_ok(),
                            addr_res.is_ok()
                        );
                    }
                }
            } else {
                // SocketAddr 形式のみ → 警告しつつスキップ（仕様は node_id@addr 推奨）
                if node.parse::<SocketAddr>().is_ok() {
                    warn!(
                        "Bootstrap node '{}' lacks NodeId; expected '<node_id>@<host:port>'. Skipping.",
                        node
                    );
                } else {
                    debug!("Unrecognized bootstrap node entry: {}", node);
                }
            }
        }

        out
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

/// NodeId を含むブートストラップノードを取得（NodeAddr）。
/// NodeId がないエントリは警告してスキップする。
pub fn load_bootstrap_node_addrs() -> Result<Vec<NodeAddr>, AppError> {
    let env = get_current_environment();
    info!("Loading bootstrap NodeAddrs for environment: {}", env);

    let config_path = "bootstrap_nodes.json";
    let config = if Path::new(config_path).exists() {
        BootstrapConfig::load_from_file(config_path)?
    } else {
        info!("Bootstrap config file not found, using defaults");
        BootstrapConfig::default_config()
    };

    let addrs = config.get_node_addrs_with_id(&env);
    if addrs.is_empty() {
        warn!("No valid NodeId@Addr bootstrap entries for environment: {}", env);
    } else {
        info!("Loaded {} NodeId@Addr bootstrap entries", addrs.len());
    }
    Ok(addrs)
}

/// 検証: JSONのノード配列のうち、NodeId@Addr と SocketAddr の件数をカウントしてログ出力
pub fn validate_bootstrap_config() -> Result<(), AppError> {
    let env = get_current_environment();
    let config_path = "bootstrap_nodes.json";
    let config = if Path::new(config_path).exists() {
        BootstrapConfig::load_from_file(config_path)?
    } else {
        BootstrapConfig::default_config()
    };

    let nodes = config.get_nodes(&env);
    let mut with_id = 0usize;
    let mut socket_only = 0usize;
    let mut invalid = 0usize;

    for node in nodes {
        if let Some((id_part, addr_part)) = node.split_once('@') {
            if NodeId::from_str(id_part).is_ok() && addr_part.parse::<SocketAddr>().is_ok() {
                with_id += 1;
            } else {
                invalid += 1;
            }
        } else if node.parse::<SocketAddr>().is_ok() {
            socket_only += 1;
        } else {
            invalid += 1;
        }
    }

    info!(
        "bootstrap_nodes.json validation (env={}): with_id={}, socket_only={}, invalid={}",
        env, with_id, socket_only, invalid
    );

    Ok(())
}

// =============== ユーザーUIによるブートストラップ指定 ===============

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserBootstrapConfig {
    nodes: Vec<String>,
}

fn user_config_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("kukuri");
    let _ = fs::create_dir_all(&dir);
    dir.join("user_bootstrap_nodes.json")
}

/// ユーザー定義のブートストラップノード（NodeId@host:port）を保存
pub fn save_user_bootstrap_nodes(nodes: &[String]) -> Result<(), AppError> {
    let path = user_config_path();
    let cfg = UserBootstrapConfig { nodes: nodes.to_vec() };
    let json = serde_json::to_string_pretty(&cfg)
        .map_err(|e| AppError::ConfigurationError(format!("Failed to serialize user bootstrap: {}", e)))?;
    fs::write(&path, json)
        .map_err(|e| AppError::ConfigurationError(format!("Failed to write user bootstrap file: {}", e)))?;
    info!("Saved user bootstrap nodes to {:?} ({} entries)", path, nodes.len());
    Ok(())
}

/// ユーザー定義のブートストラップノードを削除
pub fn clear_user_bootstrap_nodes() -> Result<(), AppError> {
    let path = user_config_path();
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AppError::ConfigurationError(format!("Failed to remove user bootstrap file: {}", e)))?;
        info!("Removed user bootstrap config at {:?}", path);
    }
    Ok(())
}

/// ユーザー定義のブートストラップノード（文字列）を読み込み
pub fn load_user_bootstrap_nodes() -> Vec<String> {
    let path = user_config_path();
    if !path.exists() {
        return vec![];
    }
    match fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<UserBootstrapConfig>(&content) {
            Ok(cfg) => cfg.nodes,
            Err(e) => {
                warn!("Invalid user bootstrap json: {}", e);
                vec![]
            }
        },
        Err(e) => {
            debug!("Failed to read user bootstrap file: {}", e);
            vec![]
        }
    }
}

/// ユーザー定義のブートストラップノード（NodeAddr）
pub fn load_user_bootstrap_node_addrs() -> Vec<NodeAddr> {
    let mut out = Vec::new();
    for node in load_user_bootstrap_nodes() {
        if let Some((id_part, addr_part)) = node.split_once('@') {
            match (NodeId::from_str(id_part), addr_part.parse::<SocketAddr>()) {
                (Ok(node_id), Ok(sock)) => out.push(NodeAddr::new(node_id).with_direct_addresses([sock])),
                _ => debug!("Invalid user bootstrap entry: {}", node),
            }
        } else {
            debug!("Skipping SocketAddr-only user bootstrap entry: {}", node);
        }
    }
    out
}
