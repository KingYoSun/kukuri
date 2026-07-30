//! provider 設定と operator-owned credentials の読み込み（#420、#391 の方式を踏襲）。
//!
//! credentials は env（名前は設定で差し替え可能）からのみ読む。値を repository / config /
//! log に置かない。エラーは env の**名前**だけを含み、値は決して含まない。
//!
//! Project Arachnid Shield（#391）との違い: OpenAI-compatible endpoint は self-host の
//! 無認証構成があり得るため、**API key は optional**（env 未設定なら Authorization ヘッダを
//! 付けない）。endpoint と model は operator が必ず指定する。

use std::time::Duration;

use thiserror::Error;

/// OpenAI-compatible API の base URL を読む env（必須。self-host / 外部を問わない）。
pub const API_BASE_URL_ENV: &str = "COMMUNITY_NODE_VLM_API_BASE_URL";
/// API key を読む env の既定名（optional。未設定なら無認証で接続する）。
pub const DEFAULT_API_KEY_ENV: &str = "COMMUNITY_NODE_VLM_API_KEY";
/// モデル名を読む env（必須。例: `inclusionAI/SingGuard-2b`）。
pub const MODEL_ENV: &str = "COMMUNITY_NODE_VLM_MODEL";
/// HTTP timeout（秒）を上書きする env。
pub const API_TIMEOUT_SECS_ENV: &str = "COMMUNITY_NODE_VLM_API_TIMEOUT_SECS";
/// 応答形式（`json` / `guard`）を選ぶ env。既定は `json`。
pub const RESPONSE_FORMAT_ENV: &str = "COMMUNITY_NODE_VLM_RESPONSE_FORMAT";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// 設定・credentials 読み込みのエラー。
///
/// env の名前のみを含む。credential の値・部分文字列を含めてはならない。
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum VlmConfigError {
    #[error(
        "environment variable `{env}` is required for the openai-compatible-vlm provider \
         but is missing or empty"
    )]
    MissingEnv { env: String },
    #[error("environment variable `{env}` has an invalid value: {detail}")]
    InvalidEnv { env: String, detail: String },
    #[error("failed to construct the openai-compatible-vlm HTTP client: {detail}")]
    Client { detail: String },
}

/// VLM 応答の解釈方式。
///
/// - `Json`: system prompt で厳密 JSON を要求する汎用モード（ADR 0028 §2.1 の既定想定）。
/// - `Guard`: guard 系モデル（SingGuard 等）向け。1 行目 `safe` / `unsafe` +
///   `<answer>` カテゴリタグを解析し、score は先頭トークンの logprob から導く。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VlmResponseFormat {
    #[default]
    Json,
    Guard,
}

impl VlmResponseFormat {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "json" => Some(Self::Json),
            "guard" => Some(Self::Guard),
            _ => None,
        }
    }
}

/// provider の実行時設定。credential の**値**は持たない（env 名のみ）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VlmProviderConfig {
    /// OpenAI-compatible API の base URL（末尾 `/` なし）。
    pub api_base_url: String,
    /// API key を読む env 名（値が未設定なら無認証で接続する）。
    pub api_key_env: String,
    /// chat/vision に使うモデル名。
    pub model: String,
    /// 応答の解釈方式。
    pub response_format: VlmResponseFormat,
    /// HTTP timeout。
    pub timeout: Duration,
}

impl VlmProviderConfig {
    /// env から設定を組み立てる。base URL と model は必須（operator-owned。既定値を持たない）。
    ///
    /// credentials（API key の値）はここでは読まない（[`VlmProviderConfig::load_credentials`]）。
    pub fn from_env() -> Result<Self, VlmConfigError> {
        let api_base_url =
            non_empty_env(API_BASE_URL_ENV).ok_or_else(|| VlmConfigError::MissingEnv {
                env: API_BASE_URL_ENV.to_string(),
            })?;
        let model = non_empty_env(MODEL_ENV).ok_or_else(|| VlmConfigError::MissingEnv {
            env: MODEL_ENV.to_string(),
        })?;
        let mut config = Self {
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            api_key_env: DEFAULT_API_KEY_ENV.to_string(),
            model,
            response_format: VlmResponseFormat::default(),
            timeout: DEFAULT_TIMEOUT,
        };
        if let Some(format) = non_empty_env(RESPONSE_FORMAT_ENV) {
            config.response_format =
                VlmResponseFormat::parse(&format).ok_or_else(|| VlmConfigError::InvalidEnv {
                    env: RESPONSE_FORMAT_ENV.to_string(),
                    detail: "expected `json` or `guard`".to_string(),
                })?;
        }
        if let Some(timeout) = non_empty_env(API_TIMEOUT_SECS_ENV) {
            let secs: u64 = timeout.parse().map_err(|_| VlmConfigError::InvalidEnv {
                env: API_TIMEOUT_SECS_ENV.to_string(),
                detail: "expected a positive integer (seconds)".to_string(),
            })?;
            if secs == 0 {
                return Err(VlmConfigError::InvalidEnv {
                    env: API_TIMEOUT_SECS_ENV.to_string(),
                    detail: "timeout must be greater than zero".to_string(),
                });
            }
            config.timeout = Duration::from_secs(secs);
        }
        Ok(config)
    }

    /// 設定された env 名から credentials を読む。
    ///
    /// API key の env が未設定・空なら **無認証**（`api_key = None`）として扱う
    /// （self-host の OpenAI-compatible endpoint は認証を持たないことがある）。
    pub fn load_credentials(&self) -> VlmCredentials {
        VlmCredentials {
            api_key: non_empty_env(&self.api_key_env),
        }
    }
}

/// operator-owned credentials。`Debug` は値を出さない。
#[derive(Clone, Default, PartialEq, Eq)]
pub struct VlmCredentials {
    pub(crate) api_key: Option<String>,
}

impl VlmCredentials {
    /// 明示的な値から組み立てる（テスト / env 以外の secret 供給経路用）。
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
        }
    }

    /// 無認証（self-host endpoint 用）。
    pub fn anonymous() -> Self {
        Self { api_key: None }
    }
}

impl std::fmt::Debug for VlmCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VlmCredentials")
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

/// 空 / 空白のみの env は未設定として扱う。
fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
