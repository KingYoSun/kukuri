//! Persist each advisory category independently so lookup/appeal retains its own signal.
use crate::{PersistedSignal, SafetyArtifactStore, SafetyScanReport, content_advisories_for};
use anyhow::{Context, Result};
use kukuri_cn_safety::{ContentAdvisory, ProviderScanRequest};

#[derive(Default)]
pub(crate) struct RecordedSignals {
    pub primary: Option<PersistedSignal>,
    pub advisories: Vec<ContentAdvisory>,
    pub any_new: bool,
}

pub(crate) async fn record_signals(
    store: &dyn SafetyArtifactStore,
    report: &SafetyScanReport,
    request: &ProviderScanRequest,
    issuer: &str,
    author: Option<&str>,
    guard: Option<&dyn crate::ScanReferenceGuard>,
) -> Result<RecordedSignals> {
    let Some(signal) = report.risk_signal.as_ref() else {
        return Ok(RecordedSignals::default());
    };
    if !report.verdict.is_labeled_allow() {
        crate::reference_guard::check(guard).await?;
        let persisted = store
            .persist_signal(issuer, signal, author)
            .await
            .context("failed to persist safety risk signal")?;
        return Ok(RecordedSignals {
            any_new: persisted.newly_created,
            primary: Some(persisted),
            advisories: Vec::new(),
        });
    }
    let mut categories = Vec::new();
    for label in &report.verdict.advisory_labels {
        if !categories.contains(&label.category) {
            categories.push(label.category);
        }
    }
    let mut recorded = RecordedSignals::default();
    for category in categories {
        let mut signal_for_category = signal.clone();
        signal_for_category.category = category;
        signal_for_category.confidence = report
            .verdict
            .advisory_labels
            .iter()
            .filter(|label| label.category == category)
            .filter_map(|label| label.confidence)
            .max()
            .or(report.verdict.confidence);
        crate::reference_guard::check(guard).await?;
        let persisted = store
            .persist_signal(issuer, &signal_for_category, author)
            .await
            .context("failed to persist category risk signal")?;
        recorded.any_new |= persisted.newly_created;
        if let (Some(kind), Some(id)) = (request.subject_kind, request.subject_id.as_deref()) {
            let mut verdict = report.verdict.clone();
            verdict
                .advisory_labels
                .retain(|label| label.category == category);
            verdict.confidence = signal_for_category.confidence;
            recorded.advisories.extend(content_advisories_for(
                &verdict,
                kind,
                id,
                issuer,
                &persisted.id,
            ));
        }
        if recorded.primary.is_none() || category == signal.category {
            recorded.primary = Some(persisted);
        }
    }
    Ok(recorded)
}
