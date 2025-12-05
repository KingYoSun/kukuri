//! Bootstrap node configuration helpers
use crate::shared::config::BootstrapSource;
use crate::shared::error::AppError;
use dirs;
use iroh::{EndpointAddr, EndpointId};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::{debug, info, trace, warn};

fn find_bootstrap_config_path() -> Option<PathBuf> {
    let primary = PathBuf::from("bootstrap_nodes.json");
    if primary.exists() {
        return Some(primary);
    }
    let alt = PathBuf::from("src-tauri").join("bootstrap_nodes.json");
    if alt.exists() {
        return Some(alt);
    }
    None
}

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
    /// Load configuration from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        let content = fs::read_to_string(path).map_err(|e| {
            AppError::ConfigurationError(format!("Failed to read bootstrap config: {e}"))
        })?;

        let config: BootstrapConfig = serde_json::from_str(&content).map_err(|e| {
            AppError::ConfigurationError(format!("Failed to parse bootstrap config: {e}"))
        })?;

        Ok(config)
    }

    /// Default configuration used when no file is available
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

    /// Get node list for the given environment key
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

    /// Get SocketAddr list for the environment
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

    /// Convert entries in the form "<node_id>@<host:port>" into EndpointAddr
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
            } else if node.parse::<SocketAddr>().is_ok() {
                warn!(
                    "Bootstrap node '{}' lacks NodeId; expected '<node_id>@<host:port>'. Skipping.",
                    node
                );
            } else {
                debug!("Unrecognized bootstrap node entry: {}", node);
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

fn sanitize_bootstrap_node(entry: &str) -> Option<String> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (node_id, addr_part) = match trimmed.split_once('@') {
        Some((id, addr)) => (id.trim(), addr.trim()),
        None => return Some(trimmed.to_string()),
    };

    let mut socket_addr = match addr_part.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(err) => {
            warn!("Invalid bootstrap node '{}': {}", entry, err);
            return None;
        }
    };

    if socket_addr.ip().is_unspecified() {
        let replacement = match socket_addr {
            SocketAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            SocketAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        };
        socket_addr.set_ip(replacement);
        warn!(
            original = %entry,
            normalized = %format!("{node_id}@{socket_addr}"),
            "Bootstrap node address was unspecified; replaced with loopback"
        );
    }

    Some(format!("{node_id}@{socket_addr}"))
}

fn sanitize_bootstrap_nodes(nodes: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = nodes
        .iter()
        .filter_map(|entry| sanitize_bootstrap_node(entry))
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

/// Read bootstrap nodes from environment variable if set
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

/// Decide effective bootstrap nodes. Priority: env > user config > bundled file (only when ENABLE_P2P_INTEGRATION=1)
pub fn load_effective_bootstrap_nodes() -> BootstrapSelection {
    let integration_enabled = std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() == "1";

    if let Some(nodes) = load_env_bootstrap_nodes() {
        let nodes = sanitize_bootstrap_nodes(&nodes);
        trace!("Using bootstrap peers from environment variable");
        return BootstrapSelection {
            source: BootstrapSource::Env,
            nodes,
        };
    }

    let user_nodes = sanitize_bootstrap_nodes(&load_user_bootstrap_nodes());
    if !user_nodes.is_empty() {
        trace!("Using bootstrap peers from user configuration");
        return BootstrapSelection {
            source: BootstrapSource::User,
            nodes: user_nodes,
        };
    }

    if integration_enabled {
        let bundle_nodes = sanitize_bootstrap_nodes(&load_bundle_bootstrap_strings());
        if !bundle_nodes.is_empty() {
            trace!("Using bootstrap peers from bundled configuration");
            return BootstrapSelection {
                source: BootstrapSource::Bundle,
                nodes: bundle_nodes,
            };
        }
    } else {
        trace!("P2P integration disabled; skipping bundled bootstrap nodes");
    }

    BootstrapSelection {
        source: BootstrapSource::None,
        nodes: Vec::new(),
    }
}

/// Resolve current environment string
pub fn get_current_environment() -> String {
    std::env::var("KUKURI_ENV")
        .or_else(|_| std::env::var("ENVIRONMENT"))
        .unwrap_or_else(|_| "development".to_string())
}

/// Load bootstrap nodes as SocketAddr
pub fn load_bootstrap_nodes() -> Result<Vec<SocketAddr>, AppError> {
    let env = get_current_environment();
    info!("Loading bootstrap nodes for environment: {}", env);

    if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1"
        && load_env_bootstrap_nodes().is_none()
        && load_user_bootstrap_nodes().is_empty()
    {
        info!("P2P integration disabled; returning empty bootstrap node list");
        return Ok(Vec::new());
    }

    let config = if let Some(path) = find_bootstrap_config_path() {
        BootstrapConfig::load_from_file(path)?
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

/// Load bootstrap nodes as EndpointAddr (requires node id)
pub fn load_bootstrap_node_addrs() -> Result<Vec<EndpointAddr>, AppError> {
    let env = get_current_environment();
    info!("Loading bootstrap NodeAddrs for environment: {}", env);

    if std::env::var("ENABLE_P2P_INTEGRATION").unwrap_or_default() != "1"
        && load_env_bootstrap_nodes().is_none()
        && load_user_bootstrap_nodes().is_empty()
    {
        info!("P2P integration disabled; returning empty NodeAddr list");
        return Ok(Vec::new());
    }

    let config = if let Some(path) = find_bootstrap_config_path() {
        BootstrapConfig::load_from_file(path)?
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

/// Validate bootstrap_nodes.json and log counts
pub fn validate_bootstrap_config() -> Result<(), AppError> {
    let env = get_current_environment();
    let config = if let Some(path) = find_bootstrap_config_path() {
        BootstrapConfig::load_from_file(path)?
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

// =============== user-managed bootstrap config =================

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

/// Persist user-provided bootstrap nodes (node_id@host:port)
pub fn save_user_bootstrap_nodes(nodes: &[String]) -> Result<(), AppError> {
    let path = user_config_path();
    let normalized = sanitize_bootstrap_nodes(nodes);
    let cfg = UserBootstrapConfig { nodes: normalized };
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

/// Remove user bootstrap configuration
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

/// Load user bootstrap nodes (raw strings)
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

/// Load user bootstrap nodes as EndpointAddr
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

    let nodes = sanitize_bootstrap_nodes(
        &cache
            .nodes
            .into_iter()
            .map(|entry| entry.trim().to_string())
            .collect::<Vec<_>>(),
    );

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
    let normalized = sanitize_bootstrap_nodes(&info.nodes);
    save_user_bootstrap_nodes(&normalized)?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static CLI_BOOTSTRAP_ENV_GUARD: OnceLock<Mutex<()>> = OnceLock::new();

    fn lock_cli_bootstrap_env() -> MutexGuard<'static, ()> {
        CLI_BOOTSTRAP_ENV_GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("cli bootstrap env guard poisoned")
    }

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
    fn sanitize_bootstrap_nodes_rewrites_unspecified_addresses() {
        let nodes = vec![
            "node1@0.0.0.0:11223".to_string(),
            "node2@[::]:11223".to_string(),
            " node3@127.0.0.1:11223 ".to_string(),
        ];

        let normalized = sanitize_bootstrap_nodes(&nodes);

        assert!(normalized.contains(&"node1@127.0.0.1:11223".to_string()));
        assert!(normalized.contains(&"node2@[::1]:11223".to_string()));
        assert!(normalized.contains(&"node3@127.0.0.1:11223".to_string()));
        assert_eq!(normalized.len(), 3);
    }

    #[test]
    fn load_cli_bootstrap_nodes_returns_data_when_file_exists() {
        let _guard = lock_cli_bootstrap_env();
        let path = temp_cli_path("load");
        unsafe {
            std::env::set_var("KUKURI_CLI_BOOTSTRAP_PATH", &path);
        }
        let payload = r#"{"nodes":["node1@127.0.0.1:1234","node1@127.0.0.1:1234","node2@[::1]:5678"],"updated_at_ms":12345}"#;
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
        let _guard = lock_cli_bootstrap_env();
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
