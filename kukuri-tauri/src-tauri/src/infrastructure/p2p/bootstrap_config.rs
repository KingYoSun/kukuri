//! Bootstrap node configuration helpers
use crate::shared::config::BootstrapSource;
use crate::shared::error::AppError;
use dirs;
use iroh::{EndpointAddr, EndpointId};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::{debug, info, trace, warn};

use super::utils::parse_node_addr;

fn prioritize_socket_addrs(addrs: Vec<SocketAddr>) -> Vec<SocketAddr> {
    let mut unique = Vec::new();
    for addr in addrs {
        if !unique.contains(&addr) {
            unique.push(addr);
        }
    }

    let mut ipv4 = Vec::new();
    let mut other = Vec::new();
    for addr in unique {
        if addr.is_ipv4() {
            ipv4.push(addr);
        } else {
            other.push(addr);
        }
    }

    ipv4.extend(other);
    ipv4
}

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
            match resolve_socket_addrs(&node) {
                Ok(mut resolved) => addrs.append(&mut resolved),
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
            match parse_node_addr(&node) {
                Ok(node_addr) => out.push(node_addr),
                Err(err) => {
                    if resolve_socket_addr(&node).is_ok() {
                        warn!(
                            "Bootstrap node '{}' lacks NodeId; expected '<node_id>@<host:port>' or relay hint format. Skipping.",
                            node
                        );
                    } else {
                        debug!("Unrecognized bootstrap node entry '{}': {}", node, err);
                    }
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

fn resolve_socket_addrs(raw: &str) -> Result<Vec<SocketAddr>, String> {
    let trimmed = raw.trim();
    if let Ok(socket_addr) = trimmed.parse::<SocketAddr>() {
        return Ok(vec![socket_addr]);
    }

    let (host, port_raw) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| format!("Invalid socket address `{raw}`"))?;
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return Err(format!("Invalid host in socket address `{raw}`"));
    }
    let port: u16 = port_raw
        .trim()
        .parse()
        .map_err(|e| format!("Invalid port `{port_raw}`: {e}"))?;

    if host.eq_ignore_ascii_case("localhost") {
        return Ok(vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)]);
    }

    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve host `{host}`: {e}"))?
        .collect();
    let prioritized = prioritize_socket_addrs(addrs);
    if prioritized.is_empty() {
        return Err(format!(
            "Resolved host `{host}` but no socket addresses were returned"
        ));
    }

    Ok(prioritized)
}

fn resolve_socket_addr(raw: &str) -> Result<SocketAddr, String> {
    resolve_socket_addrs(raw)?
        .into_iter()
        .next()
        .ok_or_else(|| format!("Resolved host `{raw}` but no socket addresses were returned"))
}

fn normalize_host_port_for_storage(entry: &str, value: &str) -> Option<String> {
    let trimmed = value.trim();
    let (host_raw, port_raw) = trimmed.rsplit_once(':')?;
    let host_raw = host_raw
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if host_raw.is_empty() {
        warn!("Invalid bootstrap node '{}': missing host", entry);
        return None;
    }

    let port_raw = port_raw.trim();
    let port = match port_raw.parse::<u16>() {
        Ok(port) => port,
        Err(err) => {
            warn!(
                "Invalid bootstrap node '{}': invalid port '{}': {}",
                entry, port_raw, err
            );
            return None;
        }
    };

    let normalized_host = if host_raw.eq_ignore_ascii_case("localhost") {
        Ipv4Addr::LOCALHOST.to_string()
    } else if let Ok(ip) = IpAddr::from_str(host_raw) {
        if ip.is_unspecified() {
            let replacement = match ip {
                IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
                IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
            };
            warn!(
                original = %entry,
                normalized = %replacement,
                "Bootstrap node address was unspecified; replaced with loopback"
            );
            replacement.to_string()
        } else {
            ip.to_string()
        }
    } else {
        host_raw.to_string()
    };

    if normalized_host.contains(':') {
        Some(format!("[{normalized_host}]:{port}"))
    } else {
        Some(format!("{normalized_host}:{port}"))
    }
}

fn normalize_extended_hint(entry: &str) -> Option<String> {
    let mut segments = entry
        .split('|')
        .map(|segment| segment.trim())
        .filter(|segment| !segment.is_empty());
    let Some(first_segment) = segments.next() else {
        warn!("Invalid bootstrap node '{}': missing node id", entry);
        return None;
    };

    let (node_id, mut addr_hint) = if let Some((node_id, addr)) = first_segment.split_once('@') {
        let node_id = node_id.trim();
        if node_id.is_empty() {
            warn!("Invalid bootstrap node '{}': missing node id", entry);
            return None;
        }
        if EndpointId::from_str(node_id).is_err() {
            warn!("Invalid bootstrap node '{}': invalid node id", entry);
            return None;
        }
        let addr_hint = normalize_host_port_for_storage(entry, addr)?;
        (node_id.to_string(), Some(addr_hint))
    } else {
        let node_id = first_segment.trim();
        if node_id.is_empty() {
            warn!("Invalid bootstrap node '{}': missing node id", entry);
            return None;
        }
        if EndpointId::from_str(node_id).is_err() {
            warn!("Invalid bootstrap node '{}': invalid node id", entry);
            return None;
        }
        (node_id.to_string(), None)
    };

    let mut relay_urls = Vec::new();
    for segment in segments {
        let Some((raw_key, raw_value)) = segment.split_once('=') else {
            warn!(
                "Invalid bootstrap node '{}': invalid hint segment '{}'",
                entry, segment
            );
            return None;
        };
        let key = raw_key.trim().to_ascii_lowercase();
        let value = raw_value.trim();
        if value.is_empty() {
            warn!(
                "Invalid bootstrap node '{}': empty value in segment '{}'",
                entry, segment
            );
            return None;
        }

        match key.as_str() {
            "relay" | "relay_url" => {
                let relay_url = match iroh::RelayUrl::from_str(value) {
                    Ok(relay_url) => relay_url,
                    Err(err) => {
                        warn!(
                            "Invalid bootstrap node '{}': relay url '{}' parse failed: {}",
                            entry, value, err
                        );
                        return None;
                    }
                };
                relay_urls.push(relay_url.to_string());
            }
            "addr" | "ip" => {
                addr_hint = normalize_host_port_for_storage(entry, value);
                if addr_hint.is_none() {
                    return None;
                }
            }
            "node" | "node_id" => {
                if value != node_id {
                    warn!(
                        "Invalid bootstrap node '{}': node id segment '{}' does not match '{}'",
                        entry, value, node_id
                    );
                    return None;
                }
            }
            _ => {
                warn!(
                    "Invalid bootstrap node '{}': unsupported hint key '{}'",
                    entry, key
                );
                return None;
            }
        }
    }

    if relay_urls.is_empty() && addr_hint.is_none() {
        warn!(
            "Invalid bootstrap node '{}': hint must include relay and/or addr",
            entry
        );
        return None;
    }

    let mut hint = node_id;
    for relay_url in relay_urls {
        hint.push_str("|relay=");
        hint.push_str(&relay_url);
    }
    if let Some(addr) = addr_hint {
        hint.push_str("|addr=");
        hint.push_str(&addr);
    }

    Some(hint)
}

fn sanitize_bootstrap_node(entry: &str) -> Option<String> {
    let trimmed = entry.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains('|') {
        return normalize_extended_hint(trimmed);
    }

    let (node_id, addr_part) = match trimmed.split_once('@') {
        Some((id, addr)) => (id.trim(), addr.trim()),
        None => return Some(trimmed.to_string()),
    };
    let normalized_addr = normalize_host_port_for_storage(entry, addr_part)?;
    Some(format!("{node_id}@{normalized_addr}"))
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
    let relay_urls: Vec<_> = node_addr
        .relay_urls()
        .map(|relay_url| relay_url.to_string())
        .collect();
    let mut hints = Vec::new();

    if relay_urls.is_empty() {
        if direct.is_empty() {
            return vec![node_id];
        }
        return direct
            .into_iter()
            .map(|addr| format!("{node_id}@{addr}"))
            .collect();
    }

    if direct.is_empty() {
        for relay_url in relay_urls {
            hints.push(format!("{node_id}|relay={relay_url}"));
        }
        return hints;
    }

    for addr in &direct {
        for relay_url in &relay_urls {
            hints.push(format!("{node_id}|relay={relay_url}|addr={addr}"));
        }
        hints.push(format!("{node_id}@{addr}"));
    }

    hints
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
            if EndpointId::from_str(id_part).is_ok() && resolve_socket_addr(addr_part).is_ok() {
                with_id += 1;
            } else {
                invalid += 1;
            }
        } else if resolve_socket_addr(&node).is_ok() {
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
        match parse_node_addr(&node) {
            Ok(node_addr) => out.push(node_addr),
            Err(err) => {
                debug!("Invalid user bootstrap entry '{}': {}", node, err);
            }
        }
    }
    out
}

const P2P_BOOTSTRAP_PATH_ENV: &str = "KUKURI_P2P_BOOTSTRAP_PATH";
const LEGACY_CLI_BOOTSTRAP_PATH_ENV: &str = "KUKURI_CLI_BOOTSTRAP_PATH";

fn default_bootstrap_export_path(file_name: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kukuri")
        .join(file_name)
}

fn bootstrap_candidate_paths() -> Vec<PathBuf> {
    if let Ok(custom) = std::env::var(P2P_BOOTSTRAP_PATH_ENV) {
        return vec![PathBuf::from(custom)];
    }
    if let Ok(legacy) = std::env::var(LEGACY_CLI_BOOTSTRAP_PATH_ENV) {
        return vec![PathBuf::from(legacy)];
    }

    vec![
        default_bootstrap_export_path("p2p_bootstrap_nodes.json"),
        default_bootstrap_export_path("cli_bootstrap_nodes.json"),
    ]
}

pub fn load_cli_bootstrap_nodes() -> Option<CliBootstrapInfo> {
    for path in bootstrap_candidate_paths() {
        if !path.exists() {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                warn!(
                    path = %path.display(),
                    error = %err,
                    "failed to read CLI bootstrap cache"
                );
                continue;
            }
        };

        let cache: CliBootstrapCacheFile = match serde_json::from_str(&content) {
            Ok(cache) => cache,
            Err(err) => {
                warn!(
                    path = %path.display(),
                    error = %err,
                    "failed to parse CLI bootstrap cache"
                );
                continue;
            }
        };

        let nodes = sanitize_bootstrap_nodes(
            &cache
                .nodes
                .into_iter()
                .map(|entry| entry.trim().to_string())
                .collect::<Vec<_>>(),
        );

        if nodes.is_empty() {
            continue;
        }

        return Some(CliBootstrapInfo {
            nodes,
            updated_at_ms: cache.updated_at_ms,
            path,
        });
    }
    None
}

pub fn apply_cli_bootstrap_nodes() -> Result<Vec<String>, AppError> {
    let info = load_cli_bootstrap_nodes().ok_or_else(|| {
        AppError::ConfigurationError(
            "CLI bootstrap list is not available. Run `cn p2p bootstrap --export-path <path>` to export nodes.".to_string(),
        )
    })?;
    let normalized = sanitize_bootstrap_nodes(&info.nodes);
    save_user_bootstrap_nodes(&normalized)?;
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv6Addr, SocketAddrV6};
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
            "node4@localhost:11224".to_string(),
        ];

        let normalized = sanitize_bootstrap_nodes(&nodes);

        assert!(normalized.contains(&"node1@127.0.0.1:11223".to_string()));
        assert!(normalized.contains(&"node2@[::1]:11223".to_string()));
        assert!(normalized.contains(&"node3@127.0.0.1:11223".to_string()));
        assert!(normalized.contains(&"node4@127.0.0.1:11224".to_string()));
        assert_eq!(normalized.len(), 4);
    }

    #[test]
    fn sanitize_bootstrap_nodes_preserves_hostname_address() {
        let nodes = vec!["node-a@bootstrap.example.com:11223".to_string()];

        let normalized = sanitize_bootstrap_nodes(&nodes);

        assert_eq!(
            normalized,
            vec!["node-a@bootstrap.example.com:11223".to_string()]
        );
    }

    #[test]
    fn prioritize_socket_addrs_prefers_ipv4() {
        let ipv6 = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 11223, 0, 0));
        let ipv4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 11223);
        let prioritized = super::prioritize_socket_addrs(vec![ipv6, ipv4]);
        assert_eq!(prioritized, vec![ipv4, ipv6]);
    }

    #[test]
    fn sanitize_bootstrap_nodes_keeps_extended_relay_hint() {
        let nodes = vec![
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example|addr=127.0.0.1:11223"
                .to_string(),
        ];

        let normalized = sanitize_bootstrap_nodes(&nodes);
        assert_eq!(normalized.len(), 1);
        assert_eq!(
            normalized[0],
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example/|addr=127.0.0.1:11223"
        );
    }

    #[test]
    fn sanitize_bootstrap_nodes_keeps_extended_relay_hint_hostname() {
        let nodes = vec![
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example|addr=bootstrap.example.com:11223"
                .to_string(),
        ];

        let normalized = sanitize_bootstrap_nodes(&nodes);
        assert_eq!(normalized.len(), 1);
        assert_eq!(
            normalized[0],
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example/|addr=bootstrap.example.com:11223"
        );
    }

    #[test]
    fn sanitize_bootstrap_nodes_keeps_relay_only_hint() {
        let nodes = vec![
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example"
                .to_string(),
        ];

        let normalized = sanitize_bootstrap_nodes(&nodes);
        assert_eq!(normalized.len(), 1);
        assert_eq!(
            normalized[0],
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef|relay=https://relay.example/"
        );
    }

    #[test]
    fn load_cli_bootstrap_nodes_returns_data_when_file_exists() {
        let _guard = lock_cli_bootstrap_env();
        let path = temp_cli_path("load");
        unsafe {
            std::env::set_var(P2P_BOOTSTRAP_PATH_ENV, &path);
        }
        let payload = r#"{"nodes":["node1@127.0.0.1:1234","node1@127.0.0.1:1234","node2@[::1]:5678"],"updated_at_ms":12345}"#;
        fs::write(&path, payload).expect("write cli bootstrap cache");

        let info = load_cli_bootstrap_nodes().expect("cli bootstrap info");
        assert_eq!(info.nodes.len(), 2);
        assert_eq!(info.updated_at_ms, Some(12345));
        assert_eq!(info.path, path);

        unsafe {
            std::env::remove_var(P2P_BOOTSTRAP_PATH_ENV);
        }
        let _ = fs::remove_file(&info.path);
    }

    #[test]
    fn load_cli_bootstrap_nodes_returns_none_when_missing() {
        let _guard = lock_cli_bootstrap_env();
        let path = temp_cli_path("missing");
        unsafe {
            std::env::set_var(P2P_BOOTSTRAP_PATH_ENV, &path);
        }
        assert!(load_cli_bootstrap_nodes().is_none());
        unsafe {
            std::env::remove_var(P2P_BOOTSTRAP_PATH_ENV);
        }
    }

    #[test]
    fn load_cli_bootstrap_nodes_supports_legacy_env_var() {
        let _guard = lock_cli_bootstrap_env();
        let path = temp_cli_path("legacy-env");
        unsafe {
            std::env::set_var(LEGACY_CLI_BOOTSTRAP_PATH_ENV, &path);
        }
        let payload = r#"{"nodes":["node-legacy@127.0.0.1:4321"],"updated_at_ms":45678}"#;
        fs::write(&path, payload).expect("write legacy cli bootstrap cache");

        let info = load_cli_bootstrap_nodes().expect("legacy cli bootstrap info");
        assert_eq!(info.nodes, vec!["node-legacy@127.0.0.1:4321".to_string()]);
        assert_eq!(info.updated_at_ms, Some(45678));
        assert_eq!(info.path, path);

        unsafe {
            std::env::remove_var(LEGACY_CLI_BOOTSTRAP_PATH_ENV);
        }
        let _ = fs::remove_file(&info.path);
    }
}
