//! Shield API の最小 HTTP client（#391）。
//!
//! OpenAPI 1.1.0（`https://shield.projectarachnid.com/openapi.json`）のうち scan に必要な
//! `POST /v1/media`（media bytes 直送）と、connectivity check 用の `POST /v1/pdq` のみを扱う。
//! `POST /v1/media/submit`（analyst への提出）と `POST /v1/url` は実装しない（#391 non-goal /
//! follow-up）。
//!
//! **Match Data 非保持**: 応答から deserialize するのは `classification` / `match_type` のみ。
//! `near_match_details`・一致先 sha1/sha256・timestamp は型に存在しないため、保存・log 出力の
//! 経路が構造的に無い。エラーにも応答 body を含めない（HTTP status のみ）。

use std::collections::BTreeMap;

use serde::Deserialize;
use thiserror::Error;

use kukuri_cn_safety::ScanError;

use crate::config::{ShieldConfigError, ShieldCredentials, ShieldProviderConfig};

/// Shield の classification。
///
/// 未知の値（API 側の将来拡張）は deserialize エラー → `ShieldError::Protocol` → fail-closed
/// に倒れる（勝手に安全側でない解釈をしない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShieldClassification {
    Csam,
    HarmfulAbusiveMaterial,
    Test,
    NoKnownMatch,
}

/// 一致種別。`exact` = 既知 hash 完全一致、`near` = PDQ 知覚ハッシュ近傍一致。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShieldMatchType {
    Exact,
    Near,
}

/// scan 判定に使う最小の応答。Match Data（近傍一致の詳細・一致先 hash 等）は含まない。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct ShieldScanResult {
    pub classification: ShieldClassification,
    #[serde(default)]
    pub match_type: Option<ShieldMatchType>,
}

/// `POST /v1/pdq` の応答（hash ごとの判定のみ）。
#[derive(Debug, Deserialize)]
struct PdqResponse {
    scanned_hashes: BTreeMap<String, ShieldScanResult>,
}

/// client のエラー。応答 body・credential 値を含めない。
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ShieldError {
    #[error(
        "project arachnid shield rejected the configured credentials (HTTP {status}); \
         verify the operator account credentials"
    )]
    Unauthorized { status: u16 },
    #[error("project arachnid shield returned HTTP {status}")]
    Http { status: u16 },
    #[error("project arachnid shield request timed out")]
    Timeout,
    #[error("failed to reach project arachnid shield: {detail}")]
    Network { detail: String },
    #[error("unexpected response from project arachnid shield: {detail}")]
    Protocol { detail: String },
}

impl From<ShieldError> for ScanError {
    /// fail-closed 写像。どの variant も `allow` に落ちる `ScanOutcome` へは写像されない
    /// （`Unavailable` → `ScanOutcome::Unavailable`、`Timeout` / `Protocol` → `Failed`）。
    fn from(error: ShieldError) -> Self {
        match error {
            ShieldError::Unauthorized { .. } | ShieldError::Network { .. } => {
                ScanError::Unavailable(error.to_string())
            }
            // 429 / 5xx は一時的な provider 側の不調として Unavailable、その他は Protocol。
            ShieldError::Http { status } if status == 429 || status >= 500 => {
                ScanError::Unavailable(error.to_string())
            }
            ShieldError::Http { .. } => ScanError::Protocol(error.to_string()),
            ShieldError::Timeout => ScanError::Timeout(error.to_string()),
            ShieldError::Protocol { .. } => ScanError::Protocol(error.to_string()),
        }
    }
}

/// Shield API client。`Debug` は credentials を出さない。
#[derive(Clone)]
pub struct ShieldClient {
    http: reqwest::Client,
    base_url: String,
    credentials: ShieldCredentials,
}

impl std::fmt::Debug for ShieldClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShieldClient")
            .field("base_url", &self.base_url)
            .field("credentials", &self.credentials)
            .finish_non_exhaustive()
    }
}

impl ShieldClient {
    /// 設定と credentials から client を組み立てる。
    pub fn new(
        config: &ShieldProviderConfig,
        credentials: ShieldCredentials,
    ) -> Result<Self, ShieldConfigError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|error| ShieldConfigError::Client {
                detail: error.to_string(),
            })?;
        Ok(Self {
            http,
            base_url: config.api_base_url.trim_end_matches('/').to_string(),
            credentials,
        })
    }

    /// 設定から credentials を env 経由で読み、client を組み立てる。
    pub fn from_config(config: &ShieldProviderConfig) -> Result<Self, ShieldConfigError> {
        let credentials = config.load_credentials()?;
        Self::new(config, credentials)
    }

    /// `POST /v1/media` — media bytes を直送して scan する。
    ///
    /// body は HTTP request としてのみ使われ、この client は保存しない。
    pub async fn scan_media(
        &self,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<ShieldScanResult, ShieldError> {
        let response = self
            .http
            .post(format!("{}/v1/media", self.base_url))
            .basic_auth(&self.credentials.username, Some(&self.credentials.password))
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes)
            .send()
            .await
            .map_err(map_transport_error)?;
        decode_response(response).await
    }

    /// `POST /v1/pdq` — PDQ hash 群を scan する（connectivity check / hash submission 用）。
    ///
    /// hash は **32 bytes の PDQ hash を base64 encode した文字列**（実 API で確認済み。
    /// hex 表現は `invalid pdq hash` の 400 になる）。
    pub async fn scan_pdq(
        &self,
        hashes: &[String],
    ) -> Result<BTreeMap<String, ShieldScanResult>, ShieldError> {
        let response = self
            .http
            .post(format!("{}/v1/pdq", self.base_url))
            .basic_auth(&self.credentials.username, Some(&self.credentials.password))
            .json(&serde_json::json!({ "hashes": hashes }))
            .send()
            .await
            .map_err(map_transport_error)?;
        let parsed: PdqResponse = decode_response(response).await?;
        Ok(parsed.scanned_hashes)
    }
}

/// 応答を検査して JSON を deserialize する。
///
/// 非 200 は body を読まずに status のみでエラー化する（Match Data・エラー body を
/// エラー文字列に取り込まない）。
async fn decode_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, ShieldError> {
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(ShieldError::Unauthorized {
            status: status.as_u16(),
        });
    }
    if !status.is_success() {
        return Err(ShieldError::Http {
            status: status.as_u16(),
        });
    }
    response
        .json::<T>()
        .await
        .map_err(|error| ShieldError::Protocol {
            // reqwest の decode エラーは応答 body を含まない（serde のエラー分類のみ）。
            // without_url で表示から URL も取り除く（表示情報の最小化）。
            detail: format!("failed to decode scan response: {}", error.without_url()),
        })
}

/// 送信段階の transport エラーを写像する。
fn map_transport_error(error: reqwest::Error) -> ShieldError {
    if error.is_timeout() {
        return ShieldError::Timeout;
    }
    ShieldError::Network {
        detail: error.without_url().to_string(),
    }
}
