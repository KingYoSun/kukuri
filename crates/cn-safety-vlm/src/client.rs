//! OpenAI-compatible API の最小 HTTP client（#420）。
//!
//! `POST {base}/v1/chat/completions` のみを扱う（chat / vision）。media は data URL
//! （`data:{content_type};base64,...`）として `image_url` パートで渡す。request body は
//! HTTP 送信にのみ使い、この client は media 本体を保存しない。
//!
//! 応答の解釈は 2 モード（[`VlmResponseFormat`]）:
//! - `json`: system prompt で厳密 JSON を要求し、`categories` / `tags` を parse する。
//! - `guard`: guard 系モデル（SingGuard 等）の「1 行目 `safe`/`unsafe` + `<answer>` カテゴリ」
//!   形式を parse し、score は先頭トークンの logprob（`exp(logprob)` を 0-100 換算）から導く。
//!
//! **機微情報の遮断**: エラーに応答 body・credential 値を含めない（HTTP status のみ）。
//! `allow` に落ちるエラー写像は存在しない（すべて fail-closed へ）。

use base64::Engine as _;
use serde::Deserialize;
use thiserror::Error;

use kukuri_cn_safety::{FetchedMedia, SafetyCategory, ScanError};

use crate::config::{VlmConfigError, VlmCredentials, VlmProviderConfig, VlmResponseFormat};

/// json モードで model に要求する応答契約。
///
/// 具体的な prompt 文言・モデル選定の標準化は ADR 0028 §4 のとおり後続スコープ。ここでは
/// parse 可能な最小契約のみを固定する。
const JSON_SYSTEM_PROMPT: &str = "You are a content moderation classifier. \
Analyze the user's content (text and/or image) and respond with ONLY a JSON object, \
no prose, no code fences, in exactly this shape: \
{\"categories\":[{\"category\":\"<one of: csam, cse, grooming, nsfw, spam, malware, phishing>\",\"score\":<0.0-1.0>}],\"tags\":[\"<descriptive search tag>\"]} \
List a category only when you detect it. Scores are your confidence (0.0-1.0). \
Tags are neutral descriptive keywords for search indexing (objects, scenery, style); \
never include category names, scores, or personal identifiers in tags.";

/// 1 回の scan 入力（text / media の最小参照。両方 `None` では呼ばない）。
#[derive(Debug, Default)]
pub struct VlmScanInput<'a> {
    pub text: Option<&'a str>,
    pub media: Option<&'a FetchedMedia>,
}

/// 検知 1 件（category + 確信度 0-100）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VlmDetection {
    pub category: SafetyCategory,
    pub score: u8,
}

/// moderation 応答の解釈結果。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VlmModeration {
    pub detections: Vec<VlmDetection>,
    /// descriptive 検索タグ（guard モードでは常に空）。
    pub tags: Vec<String>,
}

/// client のエラー。応答 body・credential 値を含めない。
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum VlmError {
    #[error(
        "the openai-compatible vlm endpoint rejected the configured credentials (HTTP {status}); \
         verify the operator API key"
    )]
    Unauthorized { status: u16 },
    #[error("the openai-compatible vlm endpoint returned HTTP {status}")]
    Http { status: u16 },
    #[error("the openai-compatible vlm request timed out")]
    Timeout,
    #[error("failed to reach the openai-compatible vlm endpoint: {detail}")]
    Network { detail: String },
    #[error("unexpected response from the openai-compatible vlm endpoint: {detail}")]
    Protocol { detail: String },
}

impl From<VlmError> for ScanError {
    /// fail-closed 写像。どの variant も `allow` に落ちる `ScanOutcome` へは写像されない
    /// （`Unavailable` → `ScanOutcome::Unavailable`、`Timeout` / `Protocol` → `Failed`）。
    fn from(error: VlmError) -> Self {
        match error {
            VlmError::Unauthorized { .. } | VlmError::Network { .. } => {
                ScanError::Unavailable(error.to_string())
            }
            // 429 / 5xx は一時的な endpoint 側の不調として Unavailable、その他は Protocol。
            VlmError::Http { status } if status == 429 || status >= 500 => {
                ScanError::Unavailable(error.to_string())
            }
            VlmError::Http { .. } => ScanError::Protocol(error.to_string()),
            VlmError::Timeout => ScanError::Timeout(error.to_string()),
            VlmError::Protocol { .. } => ScanError::Protocol(error.to_string()),
        }
    }
}

/// OpenAI-compatible chat client。`Debug` は credentials を出さない。
#[derive(Clone)]
pub struct VlmClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
    response_format: VlmResponseFormat,
    credentials: VlmCredentials,
}

impl std::fmt::Debug for VlmClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VlmClient")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("response_format", &self.response_format)
            .field("credentials", &self.credentials)
            .finish_non_exhaustive()
    }
}

impl VlmClient {
    /// 設定と credentials から client を組み立てる。
    pub fn new(
        config: &VlmProviderConfig,
        credentials: VlmCredentials,
    ) -> Result<Self, VlmConfigError> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|error| VlmConfigError::Client {
                detail: error.to_string(),
            })?;
        Ok(Self {
            http,
            base_url: config.api_base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            response_format: config.response_format,
            credentials,
        })
    }

    /// 設定から credentials を env 経由で読み、client を組み立てる。
    pub fn from_config(config: &VlmProviderConfig) -> Result<Self, VlmConfigError> {
        let credentials = config.load_credentials();
        Self::new(config, credentials)
    }

    /// `POST /v1/chat/completions` で text / media を 1 回の scan として moderation する。
    pub async fn moderate(&self, input: &VlmScanInput<'_>) -> Result<VlmModeration, VlmError> {
        let body = self.build_request_body(input);
        let mut request = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&body);
        // API key が構成されている場合のみ Bearer を付ける（self-host の無認証構成を許容）。
        if let Some(api_key) = &self.credentials.api_key {
            request = request.bearer_auth(api_key);
        }
        let response = request.send().await.map_err(map_transport_error)?;
        let chat: ChatResponse = decode_response(response).await?;
        let choice = chat
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| VlmError::Protocol {
                detail: "chat completion returned no choices".to_string(),
            })?;
        match self.response_format {
            VlmResponseFormat::Json => parse_json_moderation(&choice),
            VlmResponseFormat::Guard => parse_guard_moderation(&choice),
        }
    }

    fn build_request_body(&self, input: &VlmScanInput<'_>) -> serde_json::Value {
        let user_content = build_user_content(input);
        let mut body = serde_json::json!({
            "model": self.model,
            "temperature": 0,
            "max_tokens": 1024,
        });
        match self.response_format {
            VlmResponseFormat::Json => {
                body["messages"] = serde_json::json!([
                    { "role": "system", "content": JSON_SYSTEM_PROMPT },
                    { "role": "user", "content": user_content },
                ]);
            }
            VlmResponseFormat::Guard => {
                // guard 系モデルは chat template に判定手順が組み込まれているため
                // system prompt を付けず、先頭トークン（safe/unsafe）の logprob を要求する。
                body["messages"] = serde_json::json!([
                    { "role": "user", "content": user_content },
                ]);
                body["logprobs"] = serde_json::json!(true);
                body["top_logprobs"] = serde_json::json!(5);
            }
        }
        body
    }
}

/// user message の content を組み立てる。media がある場合は multi-part（vision）。
fn build_user_content(input: &VlmScanInput<'_>) -> serde_json::Value {
    match input.media {
        Some(media) => {
            let data_url = format!(
                "data:{};base64,{}",
                media.content_type,
                base64::engine::general_purpose::STANDARD.encode(&media.bytes)
            );
            let mut parts = Vec::new();
            if let Some(text) = input.text {
                parts.push(serde_json::json!({ "type": "text", "text": text }));
            }
            parts.push(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": data_url },
            }));
            serde_json::Value::Array(parts)
        }
        None => serde_json::Value::String(input.text.unwrap_or_default().to_string()),
    }
}

// --- 応答型（判定に必要な最小フィールドのみ deserialize する） ---

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    #[serde(default)]
    logprobs: Option<ChoiceLogprobs>,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoiceLogprobs {
    #[serde(default)]
    content: Option<Vec<TokenLogprob>>,
}

#[derive(Debug, Deserialize)]
struct TokenLogprob {
    token: String,
    logprob: f64,
}

// --- json モード ---

#[derive(Debug, Deserialize)]
struct JsonModeration {
    #[serde(default)]
    categories: Vec<JsonCategoryScore>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct JsonCategoryScore {
    category: String,
    score: f64,
}

fn parse_json_moderation(choice: &ChatChoice) -> Result<VlmModeration, VlmError> {
    let content = message_content(choice)?;
    let stripped = strip_code_fences(content);
    let parsed: JsonModeration =
        serde_json::from_str(stripped).map_err(|error| VlmError::Protocol {
            // serde のエラー分類のみ（応答 body を含めない）。
            detail: format!("failed to parse the json moderation response: {error}"),
        })?;
    let mut detections = Vec::new();
    for entry in parsed.categories {
        // 未知カテゴリは fail-closed（勝手に安全側でない解釈をしない。#391 と同じ流儀）。
        let category = parse_category(&entry.category).ok_or_else(|| VlmError::Protocol {
            detail: "json moderation response contained an unknown category".to_string(),
        })?;
        detections.push(VlmDetection {
            category,
            score: score_to_u8(entry.score),
        });
    }
    Ok(VlmModeration {
        detections,
        tags: parsed.tags,
    })
}

fn parse_category(raw: &str) -> Option<SafetyCategory> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "csam" => Some(SafetyCategory::Csam),
        "cse" => Some(SafetyCategory::Cse),
        "grooming" => Some(SafetyCategory::Grooming),
        "nsfw" => Some(SafetyCategory::Nsfw),
        "spam" => Some(SafetyCategory::Spam),
        "malware" => Some(SafetyCategory::Malware),
        "phishing" => Some(SafetyCategory::Phishing),
        _ => None,
    }
}

/// score（0.0-1.0）を 0-100 の u8 へ clamp して換算する。
fn score_to_u8(score: f64) -> u8 {
    let clamped = score.clamp(0.0, 1.0);
    // 0.0-1.0 に clamp 済みのため 0-100 に収まり、u8 への cast は安全。
    (clamped * 100.0).round() as u8
}

/// code fence（\`\`\`json 等）で包まれた応答から JSON 本体を取り出す。
fn strip_code_fences(content: &str) -> &str {
    let trimmed = content.trim();
    let Some(without_open) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    // 開始フェンス直後の言語タグ（`json` 等）を 1 行分読み飛ばす。
    let after_lang = match without_open.find('\n') {
        Some(pos) => &without_open[pos + 1..],
        None => without_open,
    };
    after_lang.strip_suffix("```").unwrap_or(after_lang).trim()
}

// --- guard モード ---

/// guard 系モデル（SingGuard 等）のカテゴリ文字 → kukuri の general moderation category 写像。
///
/// guard の粗いカテゴリからは critical（CSAM / CSE / grooming）を断定できないため、
/// **critical へは決して写像しない**（ADR 0028: 非決定論は suspected 止まり。critical の
/// 実機判定は json モード対応モデル側の役割）。
///
/// | guard | 内容 | 写像 |
/// |---|---|---|
/// | A | Sexual Content Risk | `Nsfw` |
/// | B | Real-World Crimes & Public Safety | `Nsfw`（一般 objectionable の傘。ADR 0027 §2.3） |
/// | C | Unethical Behavior（hate / harassment 等） | `Nsfw`（同上） |
/// | D | Cybersecurity & Information Manipulation | `Phishing` |
/// | E | Agent Safety（prompt injection / abuse） | `Spam` |
/// | F | Politically Sensitive Content | 検知にしない（政治的内容は kukuri の moderation 対象外） |
/// | G | Animal Abuse | `Nsfw`（一般 objectionable の傘） |
fn guard_category(letter: char) -> Option<SafetyCategory> {
    match letter.to_ascii_uppercase() {
        'A' | 'B' | 'C' | 'G' => Some(SafetyCategory::Nsfw),
        'D' => Some(SafetyCategory::Phishing),
        'E' => Some(SafetyCategory::Spam),
        'F' => None,
        _ => None,
    }
}

fn parse_guard_moderation(choice: &ChatChoice) -> Result<VlmModeration, VlmError> {
    let content = message_content(choice)?;
    let verdict = content
        .trim_start()
        .lines()
        .next()
        .map(|line| line.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match verdict.as_str() {
        // safe = この scan での検知なし（NoKnownMatch と同様、safe の証明として扱うのは router）。
        "safe" => Ok(VlmModeration::default()),
        "unsafe" => {
            let letter = parse_answer_letter(content)?;
            let Some(category) = guard_category(letter) else {
                // F（政治的内容）等、kukuri の moderation 対象にしないカテゴリ。検知なし。
                return Ok(VlmModeration::default());
            };
            Ok(VlmModeration {
                detections: vec![VlmDetection {
                    category,
                    score: guard_unsafe_score(choice),
                }],
                // guard 形式は descriptive タグを生成しない。
                tags: Vec::new(),
            })
        }
        _ => Err(VlmError::Protocol {
            detail: "guard response did not start with a safe/unsafe verdict".to_string(),
        }),
    }
}

/// `<answer>D. ...</answer>` からカテゴリ文字を取り出す。
fn parse_answer_letter(content: &str) -> Result<char, VlmError> {
    let missing = || VlmError::Protocol {
        detail: "guard response is unsafe but has no parsable <answer> category".to_string(),
    };
    let start = content.rfind("<answer>").ok_or_else(missing)?;
    let after = &content[start + "<answer>".len()..];
    let inner = match after.find("</answer>") {
        Some(end) => &after[..end],
        None => after,
    };
    inner
        .trim()
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic())
        .ok_or_else(missing)
}

/// unsafe verdict の確信度。先頭トークン（`unsafe`）の logprob から `exp(logprob)` を
/// 0-100 に換算する。logprobs が無い場合は保守的に 100（fail 側に倒す）。
fn guard_unsafe_score(choice: &ChatChoice) -> u8 {
    let Some(first) = choice
        .logprobs
        .as_ref()
        .and_then(|lp| lp.content.as_ref())
        .and_then(|tokens| tokens.first())
    else {
        return 100;
    };
    if !first.token.trim().eq_ignore_ascii_case("unsafe") {
        return 100;
    }
    score_to_u8(first.logprob.exp())
}

fn message_content(choice: &ChatChoice) -> Result<&str, VlmError> {
    choice
        .message
        .content
        .as_deref()
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| VlmError::Protocol {
            detail: "chat completion message had no content".to_string(),
        })
}

/// 応答を検査して JSON を deserialize する。
///
/// 非 200 は body を読まずに status のみでエラー化する（応答 body をエラー文字列に
/// 取り込まない）。
async fn decode_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, VlmError> {
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(VlmError::Unauthorized {
            status: status.as_u16(),
        });
    }
    if !status.is_success() {
        return Err(VlmError::Http {
            status: status.as_u16(),
        });
    }
    response
        .json::<T>()
        .await
        .map_err(|error| VlmError::Protocol {
            // reqwest の decode エラーは応答 body を含まない（serde のエラー分類のみ）。
            // without_url で表示から URL も取り除く（表示情報の最小化）。
            detail: format!(
                "failed to decode the chat response: {}",
                error.without_url()
            ),
        })
}

/// 送信段階の transport エラーを写像する。
fn map_transport_error(error: reqwest::Error) -> VlmError {
    if error.is_timeout() {
        return VlmError::Timeout;
    }
    VlmError::Network {
        detail: error.without_url().to_string(),
    }
}
