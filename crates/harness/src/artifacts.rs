use crate::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessResult {
    pub status: HarnessStatus,
    pub scenario: String,
    pub steps: Vec<StepResult>,
    pub artifacts: Vec<String>,
    pub metrics_snapshot: Option<SyncStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessStatus {
    Pass,
    Fail,
    Flaky,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResult {
    pub action: String,
    pub duration_ms: u128,
}

pub(crate) fn push_named_step(steps: &mut Vec<StepResult>, action: &str, started_at: Instant) {
    steps.push(StepResult {
        action: action.to_string(),
        duration_ms: started_at.elapsed().as_millis(),
    });
}

pub(crate) fn write_result_artifact(
    _root: &Path,
    artifacts_dir: &Path,
    result: &HarnessResult,
) -> Result<()> {
    let payload = serde_json::to_string_pretty(result)?;
    std::fs::write(artifacts_dir.join("result.json"), payload).with_context(|| {
        format!(
            "failed to write result artifact under {}",
            artifacts_dir.display()
        )
    })
}

pub fn summarize_metrics(result: &HarnessResult) -> BTreeMap<String, String> {
    let mut metrics = BTreeMap::new();
    metrics.insert(
        "status".to_string(),
        format!("{:?}", result.status).to_lowercase(),
    );
    metrics.insert("scenario".to_string(), result.scenario.clone());
    metrics.insert("steps".to_string(), result.steps.len().to_string());
    if let Some(snapshot) = &result.metrics_snapshot {
        metrics.insert("peer_count".to_string(), snapshot.peer_count.to_string());
        metrics.insert("connected".to_string(), snapshot.connected.to_string());
    }
    metrics
}
