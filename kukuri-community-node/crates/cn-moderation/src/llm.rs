use anyhow::Result;

use crate::config::LlmRuntimeConfig;

#[derive(Clone)]
pub struct LlmRequest {
    pub event_id: String,
    pub content: String,
}

#[derive(Clone)]
pub struct LlmLabel {
    pub label: String,
    pub confidence: f64,
}

#[derive(Clone)]
pub enum LlmProviderKind {
    Disabled,
    OpenAi { api_key: Option<String> },
    Local { endpoint: Option<String> },
}

impl LlmProviderKind {
    pub async fn classify(&self, _input: &LlmRequest) -> Result<Option<LlmLabel>> {
        match self {
            LlmProviderKind::Disabled => Ok(None),
            LlmProviderKind::OpenAi { .. } => Ok(None),
            LlmProviderKind::Local { .. } => Ok(None),
        }
    }
}

pub fn build_provider(config: &LlmRuntimeConfig) -> LlmProviderKind {
    match config.provider.as_str() {
        "openai" => LlmProviderKind::OpenAi {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
        },
        "local" => LlmProviderKind::Local {
            endpoint: std::env::var("LLM_LOCAL_ENDPOINT").ok(),
        },
        _ => LlmProviderKind::Disabled,
    }
}
