//! Provider decision semantics and minimal, content-free coverage (#1060).
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDecisionBasis {
    #[default]
    ScoreThreshold,
    /// Labels contain only categories whose provider boolean was true.
    CategoryFlags,
}

impl ProviderDecisionBasis {
    pub fn is_score_threshold(&self) -> bool {
        matches!(self, Self::ScoreThreshold)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanInputKind {
    Text,
    Image,
    Video,
}

/// No source bytes, prompts, raw API response, or credentials belong here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanCoverage {
    pub input: ScanInputKind,
    pub evaluated_categories: Vec<String>,
    pub unsupported_categories: Vec<String>,
    pub frames: u16,
    pub audio_scanned: bool,
    pub preprocessing_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}
