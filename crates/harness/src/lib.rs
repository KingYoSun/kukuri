use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use kukuri_app_api::{AppService, SyncStatus};
use kukuri_store::SqliteStore;
use kukuri_transport::{FakeNetwork, FakeTransport};
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, sleep, timeout};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub name: String,
    pub fixtures: ScenarioFixtures,
    pub steps: Vec<ScenarioStep>,
    pub artifacts: ScenarioArtifacts,
    pub timeouts: ScenarioTimeouts,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioFixtures {
    pub seed: u64,
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioArtifacts {
    pub dump_logs: bool,
    pub metrics_snapshot: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioTimeouts {
    pub overall_ms: u64,
    pub step_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ScenarioStep {
    LaunchDesktop,
    SelectTopic { topic: String },
    CreatePost { content: String },
    AssertTimelineContains { text: String },
    RestartDesktop,
}

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

struct ScenarioRuntime {
    db_path: PathBuf,
    network: FakeNetwork,
    app: Option<AppService>,
    current_topic: Option<String>,
}

impl ScenarioRuntime {
    async fn launch(&mut self) -> Result<()> {
        let store = Arc::new(
            SqliteStore::connect_file(&self.db_path)
                .await
                .with_context(|| {
                    format!("failed to open scenario db {}", self.db_path.display())
                })?,
        );
        let transport = Arc::new(FakeTransport::new("desktop", self.network.clone()));
        self.app = Some(AppService::new(store, transport));
        Ok(())
    }

    fn app(&self) -> Result<&AppService> {
        self.app
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("desktop app is not running"))
    }
}

pub fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario {}", path.display()))?;
    serde_yaml::from_str(&contents).context("failed to parse scenario yaml")
}

pub async fn run_named_scenario(
    root: &Path,
    name: &str,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    let path = root
        .join("harness")
        .join("scenarios")
        .join(format!("{name}.yaml"));
    let scenario = load_scenario(&path)?;
    run_scenario(root, &scenario, artifacts_dir).await
}

pub async fn run_scenario(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    std::fs::create_dir_all(artifacts_dir)
        .with_context(|| format!("failed to create artifacts dir {}", artifacts_dir.display()))?;

    let db_path = artifacts_dir.join("scenario.db");
    if db_path.exists() {
        std::fs::remove_file(&db_path)
            .with_context(|| format!("failed to remove stale db {}", db_path.display()))?;
    }

    let mut runtime = ScenarioRuntime {
        db_path,
        network: FakeNetwork::default(),
        app: None,
        current_topic: None,
    };
    let overall_timeout = Duration::from_millis(scenario.timeouts.overall_ms);
    let step_timeout = Duration::from_millis(scenario.timeouts.step_ms);

    timeout(overall_timeout, async {
        let mut steps = Vec::new();

        for step in &scenario.steps {
            let started_at = Instant::now();
            match step {
                ScenarioStep::LaunchDesktop => runtime.launch().await?,
                ScenarioStep::SelectTopic { topic } => {
                    runtime.current_topic = Some(topic.clone());
                    let _ = runtime.app()?.list_timeline(topic, None, 50).await?;
                }
                ScenarioStep::CreatePost { content } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    runtime.app()?.create_post(&topic, content, None).await?;
                }
                ScenarioStep::AssertTimelineContains { text } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    let assertion = timeout(step_timeout, async {
                        loop {
                            let timeline = runtime.app()?.list_timeline(&topic, None, 50).await?;
                            if timeline.items.iter().any(|item| item.content == *text) {
                                return Ok::<(), anyhow::Error>(());
                            }
                            sleep(Duration::from_millis(50)).await;
                        }
                    });
                    assertion.await.context("assertion timeout")??;
                }
                ScenarioStep::RestartDesktop => {
                    runtime.app.take();
                    runtime.launch().await?;
                }
            }

            steps.push(StepResult {
                action: step_name(step).to_string(),
                duration_ms: started_at.elapsed().as_millis(),
            });
        }

        let metrics_snapshot = if scenario.artifacts.metrics_snapshot {
            Some(runtime.app()?.get_sync_status().await?)
        } else {
            None
        };
        let result = HarnessResult {
            status: HarnessStatus::Pass,
            scenario: scenario.name.clone(),
            steps,
            artifacts: vec![artifacts_dir.join("result.json").display().to_string()],
            metrics_snapshot,
        };

        let payload = serde_json::to_string_pretty(&result)?;
        std::fs::write(artifacts_dir.join("result.json"), payload)
            .with_context(|| format!("failed to write result artifact under {}", root.display()))?;
        Ok::<HarnessResult, anyhow::Error>(result)
    })
    .await
    .context("scenario exceeded overall timeout")?
}

fn step_name(step: &ScenarioStep) -> &'static str {
    match step {
        ScenarioStep::LaunchDesktop => "launch_desktop",
        ScenarioStep::SelectTopic { .. } => "select_topic",
        ScenarioStep::CreatePost { .. } => "create_post",
        ScenarioStep::AssertTimelineContains { .. } => "assert_timeline_contains",
        ScenarioStep::RestartDesktop => "restart_desktop",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn desktop_smoke_post_persist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root");
        let artifacts = root
            .join("test-results")
            .join("kukuri")
            .join("desktop-smoke-test");
        let result = run_named_scenario(root, "desktop_smoke_post_persist", &artifacts)
            .await
            .expect("scenario");

        assert_eq!(result.status, HarnessStatus::Pass);
        assert!(artifacts.join("result.json").exists());
    }
}
