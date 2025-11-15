/// ブートストラップノード設定モジュール
use crate::shared::config::BootstrapSource;
use crate::shared::error::AppError;
use dirs;
use iroh::{EndpointAddr, EndpointId};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::{debug, info, trace, warn};

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

#[derive(Debug, Clone)]
pub struct BootstrapSelection {
    pub source: BootstrapSource,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CliBootstrapInfo {
    pub nodes: Vec<String>,
    pub updated_at_ms: Option<u64>,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CliBootstrapCacheFile {
    nodes: Vec<String>,
    updated_at_ms: Option<u64>,
}

impl BootstrapConfig {
    /// 設定ファイルから読み込み
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let content = fs::read_to_string(path).map_err(|e| {
            AppError::ConfigurationError(format!("Failed to read bootstrap config: {e}"))
        })?;

        let config: BootstrapConfig = serde_json::from_str(&content).map_err(|e| {
            AppError::ConfigurationError(format!("Failed to parse bootstrap config: {e}"))
        })?;

        Ok(config)
    }

    /// デフォルト設定を取得
    pub fn default_config() -> Self {
        Self {
            development: EnvironmentConfig {
                description: "Local development bootstrap nodes".to_string(),
                nodes: vec!["localhost:11223".to_string(), "localhost:11224".to_string()],
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

    /// 形式: "<node_id>@<host:port>" のみ EndpointAddr に変換する
    /// SocketAddr のみの指定は警告を出してスキップ
    pub fn get_node_addrs_with_id(&self, environment: &str) -> Vec<EndpointAddr> {
        let nodes = self.get_nodes(environment);
        let mut out = Vec::new();

        for node in nodes {
            if let Some((id_part, addr_part)) = node.split_once('@') {
                match (
                    EndpointId::from_str(id_part),
                    addr_part.parse::<SocketAddr>(),
                ) {
                    (Ok(node_id), Ok(sock)) => {
                        out.push(EndpointAddr::new(node_id).with_ip_addr(sock));
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

fn parse_bootstrap_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 環境変数 `KUKURI_BOOTSTRAP_PEERS` に設定されたブートストラップノードを取得する
pub fn load_env_bootstrap_nodes() -> Option<Vec<String>> {
    match std::env::var("KUKURI_BOOTSTRAP_PEERS") {
        Ok(raw) => {
            let nodes = parse_bootstrap_list(&raw);
            if nodes.is_empty() { None } else { Some(nodes) }
        }
        Err(_) => None,
    }
}

fn format_node_addrs(node_addr: &EndpointAddr) -> Vec<String> {
    let node_id = node_addr.id.to_string();
    let direct: Vec<_> = node_addr.ip_addrs().cloned().collect();
    if direct.is_empty() {
        vec![node_id]
    } else {
        direct
            .into_iter()
            .map(|addr| format!("{node_id}@{addr}"))
            .collect()
    }
}

fn load_bundle_bootstrap_strings() -> Vec<String> {
    match load_bootstrap_node_addrs() {
        Ok(node_addrs) => node_addrs
            .iter()
            .flat_map(format_node_addrs)
            .collect::<Vec<_>>(),
        Err(err) => {
            warn!("Failed to load bundled bootstrap nodes: {}", err);
            Vec::new()
        }
    }
}

/// UI・設定ファイル・環境変数を考慮したブートストラップノードの決定結果を返す。
/// 優先順位: 環境変数 > ユーザー設定 > 同梱JSON > なし
pub fn load_effective_bootstrap_nodes() -> BootstrapSelection {
    if let Some(nodes) = load_env_bootstrap_nodes() {
        trace!("Using bootstrap peers from environment variable");
        return BootstrapSelection {
            source: BootstrapSource::Env,
            nodes,
        };
    }

    let user_nodes = load_user_bootstrap_nodes();
    if !user_nodes.is_empty() {
        trace!("Using bootstrap peers from user configuration");
        return BootstrapSelection {
            source: BootstrapSource::User,
            nodes: user_nodes,
        };
    }

    let bundle_nodes = load_bundle_bootstrap_strings();
    if !bundle_nodes.is_empty() {
        trace!("Using bootstrap peers from bundled configuration");
        return BootstrapSelection {
            source: BootstrapSource::Bundle,
            nodes: bundle_nodes,
        };
    }

    BootstrapSelection {
        source: BootstrapSource::None,
        nodes: Vec::new(),
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

/// NodeId を含むブートストラップノードを取得（EndpointAddr）。
/// NodeId がないエントリは警告してスキップする。
pub fn load_bootstrap_node_addrs() -> Result<Vec<EndpointAddr>, AppError> {
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
        warn!(
            "No valid NodeId@Addr bootstrap entries for environment: {}",
            env
        );
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
            if EndpointId::from_str(id_part).is_ok() && addr_part.parse::<SocketAddr>().is_ok() {
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
    let cfg = UserBootstrapConfig {
        nodes: nodes.to_vec(),
    };
    let json = serde_json::to_string_pretty(&cfg).map_err(|e| {
        AppError::ConfigurationError(format!("Failed to serialize user bootstrap: {e}"))
    })?;
    fs::write(&path, json).map_err(|e| {
        AppError::ConfigurationError(format!("Failed to write user bootstrap file: {e}"))
    })?;
    info!(
        "Saved user bootstrap nodes to {:?} ({} entries)",
        path,
        nodes.len()
    );
    Ok(())
}

/// ユーザー定義のブートストラップノードを削除
pub fn clear_user_bootstrap_nodes() -> Result<(), AppError> {
    let path = user_config_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| {
            AppError::ConfigurationError(format!("Failed to remove user bootstrap file: {e}"))
        })?;
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

/// ユーザー定義のブートストラップノード（EndpointAddr）
pub fn load_user_bootstrap_node_addrs() -> Vec<EndpointAddr> {
    let mut out = Vec::new();
    for node in load_user_bootstrap_nodes() {
        if let Some((id_part, addr_part)) = node.split_once('@') {
            match (
                EndpointId::from_str(id_part),
                addr_part.parse::<SocketAddr>(),
            ) {
                (Ok(node_id), Ok(sock)) => out.push(EndpointAddr::new(node_id).with_ip_addr(sock)),
                _ => debug!("Invalid user bootstrap entry: {}", node),
            }
        } else {
            debug!("Skipping SocketAddr-only user bootstrap entry: {}", node);
        }
    }
    out
}

fn cli_bootstrap_path() -> PathBuf {
    if let Ok(custom) = std::env::var("KUKURI_CLI_BOOTSTRAP_PATH") {
        return PathBuf::from(custom);
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kukuri")
        .join("cli_bootstrap_nodes.json")
}

pub fn load_cli_bootstrap_nodes() -> Option<CliBootstrapInfo> {
    let path = cli_bootstrap_path();
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    let cache: CliBootstrapCacheFile = serde_json::from_str(&content).ok()?;

    let mut nodes: Vec<String> = cache
        .nodes
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect();
    nodes.dedup();

    if nodes.is_empty() {
        return None;
    }

    Some(CliBootstrapInfo {
        nodes,
        updated_at_ms: cache.updated_at_ms,
        path,
    })
}

pub fn apply_cli_bootstrap_nodes() -> Result<Vec<String>, AppError> {
    let info = load_cli_bootstrap_nodes().ok_or_else(|| {
        AppError::ConfigurationError(
            "CLI bootstrap list is not available. Run kukuri-cli to export nodes.".to_string(),
        )
    })?;
    save_user_bootstrap_nodes(&info.nodes)?;
    Ok(info.nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cli_path(suffix: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "kukuri_cli_bootstrap_test_{}_{}.json",
            std::process::id(),
            suffix
        ));
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
        path
    }

    #[test]
    fn load_cli_bootstrap_nodes_returns_data_when_file_exists() {
        let path = temp_cli_path("load");
        unsafe {
            std::env::set_var("KUKURI_CLI_BOOTSTRAP_PATH", &path);
        }
        let payload = r#"{"nodes":["node1@example:1234","node1@example:1234","node2@example:5678"],"updated_at_ms":12345}"#;
        fs::write(&path, payload).expect("write cli bootstrap cache");

        let info = load_cli_bootstrap_nodes().expect("cli bootstrap info");
        assert_eq!(info.nodes.len(), 2);
        assert_eq!(info.updated_at_ms, Some(12345));
        assert_eq!(info.path, path);

        unsafe {
            std::env::remove_var("KUKURI_CLI_BOOTSTRAP_PATH");
        }
        let _ = fs::remove_file(&info.path);
    }

    #[test]
    fn load_cli_bootstrap_nodes_returns_none_when_missing() {
        let path = temp_cli_path("missing");
        unsafe {
            std::env::set_var("KUKURI_CLI_BOOTSTRAP_PATH", &path);
        }
        assert!(load_cli_bootstrap_nodes().is_none());
        unsafe {
            std::env::remove_var("KUKURI_CLI_BOOTSTRAP_PATH");
        }
    }
}
