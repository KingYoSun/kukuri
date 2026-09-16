use std::collections::BTreeMap;

use kukuri_cn_safety::{
    ProviderDecisionBasis, ProviderScanResult, SafetyCategory, SafetyLabel,
    SafetyProviderCapability, ScanCoverage, ScanError, ScanInputKind,
};
use serde::{Deserialize, Serialize};

use crate::config::invalid;

pub const CATEGORIES: &[(&str, bool)] = &[
    ("harassment", false),
    ("harassment/threatening", false),
    ("hate", false),
    ("hate/threatening", false),
    ("illicit", false),
    ("illicit/violent", false),
    ("self-harm", true),
    ("self-harm/intent", true),
    ("self-harm/instructions", true),
    ("sexual", true),
    ("sexual/minors", false),
    ("violence", true),
    ("violence/graphic", true),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryAssessment {
    pub flagged: bool,
    /// Fixed point confidence (0..10,000), never used as a decision threshold.
    pub confidence: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModerationAssessment {
    pub categories: BTreeMap<String, CategoryAssessment>,
    pub unsupported: Vec<String>,
}

impl ModerationAssessment {
    pub fn parse(bytes: &[u8], input: ScanInputKind) -> Result<Self, ScanError> {
        let body: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| invalid("invalid moderation JSON"))?;
        let results = body["results"]
            .as_array()
            .filter(|r| r.len() == 1)
            .ok_or_else(|| invalid("moderation response must contain exactly one result"))?;
        let result = &results[0];
        if body["model"].as_str().is_none_or(|m| m.is_empty()) || !result["flagged"].is_boolean() {
            return Err(invalid("missing moderation response metadata"));
        }
        let image = input != ScanInputKind::Text;
        let expected_type = if image { "image" } else { "text" };
        let mut categories = BTreeMap::new();
        let mut unsupported = Vec::new();
        for (name, supports_image) in CATEGORIES {
            let flagged = result["categories"][name]
                .as_bool()
                .ok_or_else(|| invalid("missing moderation category boolean"))?;
            let score = result["category_scores"][name]
                .as_f64()
                .filter(|s| s.is_finite() && (0.0..=1.0).contains(s))
                .ok_or_else(|| invalid("invalid moderation confidence"))?;
            let applied = result["category_applied_input_types"][name]
                .as_array()
                .ok_or_else(|| invalid("missing moderation input coverage"))?;
            if image && !supports_image {
                if !applied.is_empty() || flagged {
                    return Err(invalid("unsupported image category was asserted"));
                }
                unsupported.push((*name).to_owned());
                continue;
            }
            if applied.len() != 1 || applied[0].as_str() != Some(expected_type) {
                return Err(invalid("moderation input coverage mismatch"));
            }
            categories.insert(
                (*name).to_owned(),
                CategoryAssessment {
                    flagged,
                    confidence: (score * 10_000.0).round() as u16,
                },
            );
        }
        if result["categories"]
            .as_object()
            .is_none_or(|map| map.len() != CATEGORIES.len())
        {
            return Err(invalid("unrecognized moderation category set"));
        }
        Ok(Self {
            categories,
            unsupported,
        })
    }

    pub fn merge(&mut self, frame: Self) -> Result<(), ScanError> {
        if self.unsupported != frame.unsupported
            || self.categories.keys().ne(frame.categories.keys())
        {
            return Err(invalid("frame coverage changed within a video scan"));
        }
        for (name, next) in frame.categories {
            let current = self
                .categories
                .get_mut(&name)
                .expect("identical category keys");
            current.flagged |= next.flagged;
            current.confidence = current.confidence.max(next.confidence);
        }
        Ok(())
    }

    pub fn into_result(
        self,
        input: ScanInputKind,
        frames: u16,
        duration_ms: Option<u64>,
    ) -> ProviderScanResult {
        let capability = SafetyProviderCapability::GeneralMediaModeration;
        let mut result = ProviderScanResult::completed(crate::PROVIDER_NAME, capability);
        result.decision_basis = ProviderDecisionBasis::CategoryFlags;
        for (name, assessment) in &self.categories {
            if !assessment.flagged {
                continue;
            }
            let category = match name.as_str() {
                "sexual" => SafetyCategory::Nsfw,
                "sexual/minors" => SafetyCategory::Cse,
                _ => SafetyCategory::Objectionable,
            };
            let confidence = ((u32::from(assessment.confidence) + 50) / 100) as u8;
            // Merge only into the same product category and only flagged detections.
            if let Some(label) = result.labels.iter_mut().find(|l| l.category == category) {
                label.confidence = Some(label.confidence.unwrap_or(0).max(confidence));
            } else {
                result.labels.push(
                    SafetyLabel::new(category)
                        .with_confidence(confidence)
                        .with_provider_capability(capability),
                );
            }
        }
        result.coverage = Some(ScanCoverage {
            input,
            evaluated_categories: self.categories.into_keys().collect(),
            unsupported_categories: self.unsupported,
            frames,
            audio_scanned: false,
            preprocessing_version: crate::PREPROCESSING_VERSION.to_owned(),
            duration_ms,
        });
        result
    }
}
