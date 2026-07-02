//! provider 設定と operator-owned credentials の読み込み（#391）。
//!
//! credentials は env（名前は設定で差し替え可能）からのみ読む。値を repository / config /
//! log に置かない。エラーは env の**名前**だけを含み、値は決して含まない。

use std::time::Duration;

use thiserror::Error;

/// Shield API の既定 base URL。
pub const DEFAULT_API_BASE_URL: &str = "https://shield.projectarachnid.com";
/// credentials username を読む env の既定名（Project Arachnid 管理ページの表記に合わせる）。
pub const DEFAULT_API_USERNAME_ENV: &str = "PROJECT_ARACHNID_API_USERNAME";
/// credentials password を読む env の既定名（同上）。
pub const DEFAULT_API_PASSWORD_ENV: &str = "PROJECT_ARACHNID_API_PASSWORD";
/// base URL を上書きする env（テスト / self-host 用。未設定なら既定 URL）。
pub const API_BASE_URL_ENV: &str = "PROJECT_ARACHNID_API_BASE_URL";
/// HTTP timeout（秒）を上書きする env。
pub const API_TIMEOUT_SECS_ENV: &str = "PROJECT_ARACHNID_API_TIMEOUT_SECS";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// 設定・credentials 読み込みのエラー。
///
/// env の名前のみを含む。credential の値・部分文字列を含めてはならない。
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ShieldConfigError {
    #[error(
        "environment variable `{env}` is required for the project-arachnid-shield provider \
         but is missing or empty"
    )]
    MissingCredential { env: String },
    #[error("environment variable `{env}` has an invalid value: {detail}")]
    InvalidEnv { env: String, detail: String },
    #[error("failed to construct the project-arachnid-shield HTTP client: {detail}")]
    Client { detail: String },
}

/// provider の実行時設定。credential の**値**は持たない（env 名のみ）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShieldProviderConfig {
    /// Shield API の base URL（末尾 `/` なし）。
    pub api_base_url: String,
    /// username を読む env 名。
    pub api_username_env: String,
    /// password を読む env 名。
    pub api_password_env: String,
    /// HTTP timeout。
    pub timeout: Duration,
}

impl Default for ShieldProviderConfig {
    fn default() -> Self {
        Self {
            api_base_url: DEFAULT_API_BASE_URL.to_string(),
            api_username_env: DEFAULT_API_USERNAME_ENV.to_string(),
            api_password_env: DEFAULT_API_PASSWORD_ENV.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl ShieldProviderConfig {
    /// env（`PROJECT_ARACHNID_API_BASE_URL` / `PROJECT_ARACHNID_API_TIMEOUT_SECS`）から
    /// 上書き値を読み、設定を組み立てる。credentials はここでは読まない
    /// （[`ShieldProviderConfig::load_credentials`] が読む）。
    pub fn from_env() -> Result<Self, ShieldConfigError> {
        let mut config = Self::default();
        if let Some(base_url) = non_empty_env(API_BASE_URL_ENV) {
            config.api_base_url = base_url.trim_end_matches('/').to_string();
        }
        if let Some(timeout) = non_empty_env(API_TIMEOUT_SECS_ENV) {
            let secs: u64 = timeout.parse().map_err(|_| ShieldConfigError::InvalidEnv {
                env: API_TIMEOUT_SECS_ENV.to_string(),
                detail: "expected a positive integer (seconds)".to_string(),
            })?;
            if secs == 0 {
                return Err(ShieldConfigError::InvalidEnv {
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
    /// 欠落・空は Err（呼び出し側で起動失敗 = fail-closed）。エラーに値は含まれない。
    pub fn load_credentials(&self) -> Result<ShieldCredentials, ShieldConfigError> {
        let username = non_empty_env(&self.api_username_env).ok_or_else(|| {
            ShieldConfigError::MissingCredential {
                env: self.api_username_env.clone(),
            }
        })?;
        let password = non_empty_env(&self.api_password_env).ok_or_else(|| {
            ShieldConfigError::MissingCredential {
                env: self.api_password_env.clone(),
            }
        })?;
        Ok(ShieldCredentials { username, password })
    }
}

/// operator-owned credentials。`Debug` は値を出さない。
#[derive(Clone, PartialEq, Eq)]
pub struct ShieldCredentials {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl ShieldCredentials {
    /// 明示的な値から組み立てる（テスト / env 以外の secret 供給経路用）。
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

impl std::fmt::Debug for ShieldCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShieldCredentials")
            .field("username", &"<redacted>")
            .field("password", &"<redacted>")
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
