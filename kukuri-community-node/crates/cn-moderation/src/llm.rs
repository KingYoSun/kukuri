use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

use crate::config::LlmRuntimeConfig;

const DEFAULT_OPENAI_MODERATION_ENDPOINT: &str = "https://api.openai.com/v1/moderations";
const DEFAULT_OPENAI_MODERATION_MODEL: &str = "omni-moderation-latest";

#[derive(Clone, Debug)]
pub struct LlmRequest {
    pub event_id: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct LlmLabel {
    pub label: String,
    pub confidence: Option<f64>,
}

#[derive(Clone)]
pub enum LlmProviderKind {
    Disabled,
    OpenAi {
        client: Client,
        endpoint: String,
        model: String,
        api_key: Option<String>,
        external_send_enabled: bool,
    },
    Local {
        client: Client,
        endpoint: Option<String>,
    },
}

impl LlmProviderKind {
    pub fn source(&self) -> &'static str {
        match self {
            LlmProviderKind::Disabled => "llm:disabled",
            LlmProviderKind::OpenAi { .. } => "llm:openai",
            LlmProviderKind::Local { .. } => "llm:local",
        }
    }

    pub async fn classify(&self, input: &LlmRequest) -> Result<Option<LlmLabel>> {
        match self {
            LlmProviderKind::Disabled => Ok(None),
            LlmProviderKind::OpenAi {
                client,
                endpoint,
                model,
                api_key,
                external_send_enabled,
            } => {
                if !external_send_enabled {
                    return Ok(None);
                }
                let api_key = api_key.as_ref().ok_or_else(|| {
                    anyhow!("OPENAI_API_KEY is required when LLM provider=openai")
                })?;
                let response = client
                    .post(endpoint)
                    .bearer_auth(api_key)
                    .json(&json!({
                        "model": model,
                        "input": input.content
                    }))
                    .send()
                    .await?;
                let status = response.status();
                if !status.is_success() {
                    let body = response.text().await.unwrap_or_default();
                    return Err(anyhow!(
                        "OpenAI moderation request failed (status={}): {}",
                        status,
                        body
                    ));
                }
                let parsed: OpenAiModerationResponse = response.json().await?;
                Ok(map_openai_response(parsed))
            }
            LlmProviderKind::Local { client, endpoint } => {
                let endpoint = endpoint
                    .as_ref()
                    .ok_or_else(|| anyhow!("LLM_LOCAL_ENDPOINT is required when provider=local"))?;
                let response = client
                    .post(endpoint)
                    .json(&json!({
                        "event_id": input.event_id,
                        "input": input.content
                    }))
                    .send()
                    .await?;
                let status = response.status();
                if !status.is_success() {
                    let body = response.text().await.unwrap_or_default();
                    return Err(anyhow!(
                        "Local moderation request failed (status={}): {}",
                        status,
                        body
                    ));
                }
                let payload: Value = response.json().await?;
                Ok(parse_local_response(&payload))
            }
        }
    }
}

pub fn build_provider(config: &LlmRuntimeConfig) -> LlmProviderKind {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| Client::new());

    match config.provider.as_str() {
        "openai" => LlmProviderKind::OpenAi {
            client,
            endpoint: std::env::var("OPENAI_MODERATION_ENDPOINT")
                .unwrap_or_else(|_| DEFAULT_OPENAI_MODERATION_ENDPOINT.to_string()),
            model: std::env::var("OPENAI_MODERATION_MODEL")
                .unwrap_or_else(|_| DEFAULT_OPENAI_MODERATION_MODEL.to_string()),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            external_send_enabled: config.external_send_enabled,
        },
        "local" => LlmProviderKind::Local {
            client,
            endpoint: std::env::var("LLM_LOCAL_ENDPOINT").ok(),
        },
        _ => LlmProviderKind::Disabled,
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiModerationResponse {
    #[serde(default)]
    results: Vec<OpenAiModerationResult>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModerationResult {
    flagged: bool,
    #[serde(default)]
    categories: HashMap<String, bool>,
    #[serde(default)]
    category_scores: HashMap<String, f64>,
}

fn map_openai_response(response: OpenAiModerationResponse) -> Option<LlmLabel> {
    let result = response.results.into_iter().next()?;
    if !result.flagged {
        return None;
    }

    let mut best: Option<(String, Option<f64>)> = None;
    for (category, flagged) in &result.categories {
        if !flagged {
            continue;
        }
        let score = result.category_scores.get(category).copied();
        let label = map_openai_category(category);
        match &best {
            Some((_, Some(current_score))) if score.unwrap_or(0.0) <= *current_score => {}
            Some((_, None)) if score.is_none() => {}
            _ => best = Some((label, score)),
        }
    }

    if best.is_none() {
        for (category, score) in &result.category_scores {
            let label = map_openai_category(category);
            match &best {
                Some((_, Some(current_score))) if *score <= *current_score => {}
                _ => best = Some((label, Some(*score))),
            }
        }
    }

    let (label, confidence) = best?;
    build_label(label, confidence)
}

fn map_openai_category(category: &str) -> String {
    let normalized = category.to_lowercase();
    if normalized.starts_with("sexual") {
        "nsfw".to_string()
    } else if normalized.starts_with("harassment") || normalized.starts_with("hate") {
        "harassment".to_string()
    } else if normalized.starts_with("violence")
        || normalized.starts_with("self-harm")
        || normalized.starts_with("illicit")
    {
        "illegal".to_string()
    } else {
        "illegal".to_string()
    }
}

fn parse_local_response(payload: &Value) -> Option<LlmLabel> {
    let object = payload.as_object()?;

    if let Some(label) = object.get("label").and_then(Value::as_str) {
        let confidence = parse_confidence(object.get("confidence"));
        return build_label(label.to_string(), confidence);
    }

    if let Some(result) = object.get("result").and_then(Value::as_object) {
        if let Some(label) = result.get("label").and_then(Value::as_str) {
            let confidence = parse_confidence(result.get("confidence"));
            return build_label(label.to_string(), confidence);
        }
    }

    if let Some(categories) = object.get("categories").and_then(Value::as_object) {
        let mut best_label: Option<String> = None;
        let mut best_score: Option<f64> = None;
        for (label, value) in categories {
            let Some(score) = parse_confidence(Some(value)) else {
                continue;
            };
            if best_score.map(|current| score > current).unwrap_or(true) {
                best_score = Some(score);
                best_label = Some(label.to_string());
            }
        }
        if let Some(label) = best_label {
            return build_label(label, best_score);
        }
    }

    None
}

fn parse_confidence(value: Option<&Value>) -> Option<f64> {
    let value = value?;
    let parsed = if let Some(number) = value.as_f64() {
        Some(number)
    } else if let Some(string) = value.as_str() {
        string.parse::<f64>().ok()
    } else {
        None
    }?;
    if !parsed.is_finite() {
        return None;
    }
    Some(parsed.clamp(0.0, 1.0))
}

fn build_label(label: String, confidence: Option<f64>) -> Option<LlmLabel> {
    let normalized = label.trim().to_lowercase();
    if normalized.is_empty() || normalized == "safe" || normalized == "none" {
        return None;
    }
    Some(LlmLabel {
        label: normalized,
        confidence,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn map_openai_response_returns_none_when_not_flagged() {
        let response = OpenAiModerationResponse {
            results: vec![OpenAiModerationResult {
                flagged: false,
                categories: HashMap::new(),
                category_scores: HashMap::new(),
            }],
        };
        assert!(map_openai_response(response).is_none());
    }

    #[test]
    fn map_openai_response_maps_categories_to_internal_labels() {
        let response = OpenAiModerationResponse {
            results: vec![OpenAiModerationResult {
                flagged: true,
                categories: HashMap::from([
                    ("sexual".to_string(), true),
                    ("harassment".to_string(), true),
                ]),
                category_scores: HashMap::from([
                    ("sexual".to_string(), 0.8),
                    ("harassment".to_string(), 0.7),
                ]),
            }],
        };
        let mapped = map_openai_response(response).expect("expected label");
        assert_eq!(mapped.label, "nsfw");
        assert_eq!(mapped.confidence, Some(0.8));
    }

    #[test]
    fn parse_local_response_accepts_direct_label_shape() {
        let payload = json!({
            "label": "spam",
            "confidence": 0.93
        });
        let result = parse_local_response(&payload).expect("expected label");
        assert_eq!(result.label, "spam");
        assert_eq!(result.confidence, Some(0.93));
    }

    #[test]
    fn parse_local_response_accepts_nested_result_shape() {
        let payload = json!({
            "result": {
                "label": "harassment",
                "confidence": "0.56"
            }
        });
        let result = parse_local_response(&payload).expect("expected label");
        assert_eq!(result.label, "harassment");
        assert_eq!(result.confidence, Some(0.56));
    }

    #[test]
    fn parse_local_response_uses_highest_category_score() {
        let payload = json!({
            "categories": {
                "spam": 0.81,
                "nsfw": 0.42
            }
        });
        let result = parse_local_response(&payload).expect("expected label");
        assert_eq!(result.label, "spam");
        assert_eq!(result.confidence, Some(0.81));
    }

    #[test]
    fn parse_local_response_skips_safe_label() {
        let payload = json!({
            "label": "safe",
            "confidence": 0.99
        });
        assert!(parse_local_response(&payload).is_none());
    }
}
