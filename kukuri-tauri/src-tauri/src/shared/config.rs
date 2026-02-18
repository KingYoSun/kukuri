use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapSource {
    Env,
    User,
    Bundle,
    Fallback,
    #[default]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub network: NetworkConfig,
    pub sync: SyncConfig,
    pub storage: StorageConfig,
    pub metrics: MetricsConfig,
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
    // DHT/Discovery 関連フラグ
    pub enable_dht: bool,
    pub enable_dns: bool,
    pub enable_local: bool,
    #[serde(default)]
    pub bootstrap_source: BootstrapSource,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub interval_minutes: u64,
    pub ttl_hours: u64,
    pub score_weights: MetricsScoreWeightsConfig,
    #[serde(default)]
    pub prometheus_port: Option<u16>,
    #[serde(default)]
    pub emit_histogram: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsScoreWeightsConfig {
    pub posts: f64,
    pub unique_authors: f64,
    pub boosts: f64,
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
                enable_dht: true,
                enable_dns: true,
                enable_local: false,
                bootstrap_source: BootstrapSource::None,
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
                cache_ttl: 3600,               // 1 hour
            },
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 5,
            ttl_hours: 48,
            score_weights: MetricsScoreWeightsConfig::default(),
            prometheus_port: None,
            emit_histogram: false,
        }
    }
}

impl Default for MetricsScoreWeightsConfig {
    fn default() -> Self {
        Self {
            posts: 0.6,
            unique_authors: 0.3,
            boosts: 0.1,
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        // 既定値
        let mut cfg = Self::default();
        let mut bootstrap_source = BootstrapSource::None;

        // ネットワーク設定の環境変数反映
        if let Ok(v) = std::env::var("KUKURI_ENABLE_DHT") {
            cfg.network.enable_dht = parse_bool(&v, cfg.network.enable_dht);
        }
        if let Ok(v) = std::env::var("KUKURI_ENABLE_DNS") {
            cfg.network.enable_dns = parse_bool(&v, cfg.network.enable_dns);
        }
        if let Ok(v) = std::env::var("KUKURI_ENABLE_LOCAL") {
            cfg.network.enable_local = parse_bool(&v, cfg.network.enable_local);
        }

        if let Ok(v) = std::env::var("KUKURI_BOOTSTRAP_PEERS") {
            let peers: Vec<String> = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !peers.is_empty() {
                cfg.network.bootstrap_peers = peers;
                bootstrap_source = BootstrapSource::Env;
            }
        }

        cfg.network.bootstrap_source = bootstrap_source;

        if let Ok(v) = std::env::var("KUKURI_METRICS_ENABLED") {
            cfg.metrics.enabled = parse_bool(&v, cfg.metrics.enabled);
        }
        if let Ok(v) = std::env::var("KUKURI_METRICS_INTERVAL_MINUTES")
            && let Some(value) = parse_u64(&v)
        {
            cfg.metrics.interval_minutes = value.max(1);
        }
        if let Ok(v) = std::env::var("KUKURI_METRICS_TTL_HOURS")
            && let Some(value) = parse_u64(&v)
        {
            cfg.metrics.ttl_hours = value.max(1);
        }
        if let Ok(v) = std::env::var("KUKURI_METRICS_WEIGHT_POSTS")
            && let Some(value) = parse_f64(&v)
        {
            cfg.metrics.score_weights.posts = value.max(0.0);
        }
        if let Ok(v) = std::env::var("KUKURI_METRICS_WEIGHT_UNIQUE_AUTHORS")
            && let Some(value) = parse_f64(&v)
        {
            cfg.metrics.score_weights.unique_authors = value.max(0.0);
        }
        if let Ok(v) = std::env::var("KUKURI_METRICS_WEIGHT_BOOSTS")
            && let Some(value) = parse_f64(&v)
        {
            cfg.metrics.score_weights.boosts = value.max(0.0);
        }
        if let Ok(v) = std::env::var("KUKURI_METRICS_PROMETHEUS_PORT")
            && let Some(value) = parse_u16(&v)
        {
            cfg.metrics.prometheus_port = if value == 0 { None } else { Some(value) };
        }
        if let Ok(v) = std::env::var("KUKURI_METRICS_EMIT_HISTOGRAM") {
            cfg.metrics.emit_histogram = parse_bool(&v, cfg.metrics.emit_histogram);
        }

        cfg
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.database.max_connections == 0 {
            return Err("Database max_connections must be greater than 0".to_string());
        }
        if self.network.max_peers == 0 {
            return Err("Network max_peers must be greater than 0".to_string());
        }
        if self.metrics.enabled {
            if self.metrics.interval_minutes == 0 {
                return Err("Metrics interval_minutes must be greater than 0".to_string());
            }
            if self.metrics.ttl_hours == 0 {
                return Err("Metrics ttl_hours must be greater than 0".to_string());
            }
        }
        if let Some(port) = self.metrics.prometheus_port
            && port == 0
        {
            return Err("Metrics prometheus_port must be greater than 0".to_string());
        }
        Ok(())
    }
}

fn parse_bool(s: &str, default: bool) -> bool {
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn parse_u64(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

fn parse_f64(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

fn parse_u16(value: &str) -> Option<u16> {
    value.trim().parse::<u16>().ok()
}
