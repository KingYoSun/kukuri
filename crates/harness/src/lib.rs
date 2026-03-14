use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use kukuri_app_api::{
    AppService, CreateGameRoomInput, CreateLiveSessionInput, GameScoreView, SyncStatus,
    UpdateGameRoomInput,
};
use kukuri_core::GameRoomStatus;
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
    SelectTopic {
        topic: String,
    },
    CreatePost {
        content: String,
    },
    AssertTimelineContains {
        text: String,
    },
    CreateLiveSession {
        title: String,
        description: String,
    },
    JoinLiveSession {
        title: String,
    },
    AssertLiveViewerCount {
        title: String,
        viewer_count: usize,
    },
    EndLiveSession {
        title: String,
    },
    CreateGameRoom {
        title: String,
        description: String,
        participants: Vec<String>,
    },
    UpdateGameRoom {
        title: String,
        status: String,
        phase_label: Option<String>,
        scores: Vec<ScenarioScoreUpdate>,
    },
    AssertGameScore {
        title: String,
        label: String,
        score: i64,
    },
    RestartDesktop,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioScoreUpdate {
    pub label: String,
    pub score: i64,
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
                ScenarioStep::CreateLiveSession { title, description } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    runtime
                        .app()?
                        .create_live_session(
                            &topic,
                            CreateLiveSessionInput {
                                title: title.clone(),
                                description: description.clone(),
                            },
                        )
                        .await?;
                }
                ScenarioStep::JoinLiveSession { title } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    let session = runtime
                        .app()?
                        .list_live_sessions(&topic)
                        .await?
                        .into_iter()
                        .find(|session| session.title == *title)
                        .with_context(|| format!("live session not found: {title}"))?;
                    runtime
                        .app()?
                        .join_live_session(&topic, session.session_id.as_str())
                        .await?;
                }
                ScenarioStep::AssertLiveViewerCount { title, viewer_count } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    let expected = *viewer_count;
                    let target = title.clone();
                    let assertion = timeout(step_timeout, async {
                        loop {
                            let sessions = runtime.app()?.list_live_sessions(&topic).await?;
                            if sessions
                                .iter()
                                .any(|session| session.title == target && session.viewer_count == expected)
                            {
                                return Ok::<(), anyhow::Error>(());
                            }
                            sleep(Duration::from_millis(50)).await;
                        }
                    });
                    match assertion.await {
                        Ok(result) => result?,
                        Err(_) => {
                            let sessions = runtime.app()?.list_live_sessions(&topic).await?;
                            let observed = sessions
                                .iter()
                                .map(|session| {
                                    format!(
                                        "{}:{}:{}",
                                        session.title, session.viewer_count, session.joined_by_me
                                    )
                                })
                                .collect::<Vec<_>>();
                            anyhow::bail!(
                                "assertion timeout for live viewer count title={target} expected={expected} observed={observed:?}"
                            );
                        }
                    }
                }
                ScenarioStep::EndLiveSession { title } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    let session = runtime
                        .app()?
                        .list_live_sessions(&topic)
                        .await?
                        .into_iter()
                        .find(|session| session.title == *title)
                        .with_context(|| format!("live session not found: {title}"))?;
                    runtime
                        .app()?
                        .end_live_session(&topic, session.session_id.as_str())
                        .await?;
                }
                ScenarioStep::CreateGameRoom {
                    title,
                    description,
                    participants,
                } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    runtime
                        .app()?
                        .create_game_room(
                            &topic,
                            CreateGameRoomInput {
                                title: title.clone(),
                                description: description.clone(),
                                participants: participants.clone(),
                            },
                        )
                        .await?;
                }
                ScenarioStep::UpdateGameRoom {
                    title,
                    status,
                    phase_label,
                    scores,
                } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    let room = runtime
                        .app()?
                        .list_game_rooms(&topic)
                        .await?
                        .into_iter()
                        .find(|room| room.title == *title)
                        .with_context(|| format!("game room not found: {title}"))?;
                    let next_scores = room
                        .scores
                        .iter()
                        .map(|score| {
                            let next = scores
                                .iter()
                                .find(|update| update.label == score.label)
                                .map(|update| update.score)
                                .unwrap_or(score.score);
                            GameScoreView {
                                participant_id: score.participant_id.clone(),
                                label: score.label.clone(),
                                score: next,
                            }
                        })
                        .collect::<Vec<_>>();
                    runtime
                        .app()?
                        .update_game_room(
                            &topic,
                            room.room_id.as_str(),
                            UpdateGameRoomInput {
                                status: parse_game_status(status.as_str())?,
                                phase_label: phase_label.clone(),
                                scores: next_scores,
                            },
                        )
                        .await?;
                }
                ScenarioStep::AssertGameScore { title, label, score } => {
                    let topic = runtime
                        .current_topic
                        .clone()
                        .unwrap_or_else(|| scenario.fixtures.topic.clone());
                    let expected_title = title.clone();
                    let expected_label = label.clone();
                    let expected_score = *score;
                    let assertion = timeout(step_timeout, async {
                        loop {
                            let rooms = runtime.app()?.list_game_rooms(&topic).await?;
                            if rooms.iter().any(|room| {
                                room.title == expected_title
                                    && room.scores.iter().any(|entry| {
                                        entry.label == expected_label && entry.score == expected_score
                                    })
                            }) {
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
        ScenarioStep::CreateLiveSession { .. } => "create_live_session",
        ScenarioStep::JoinLiveSession { .. } => "join_live_session",
        ScenarioStep::AssertLiveViewerCount { .. } => "assert_live_viewer_count",
        ScenarioStep::EndLiveSession { .. } => "end_live_session",
        ScenarioStep::CreateGameRoom { .. } => "create_game_room",
        ScenarioStep::UpdateGameRoom { .. } => "update_game_room",
        ScenarioStep::AssertGameScore { .. } => "assert_game_score",
        ScenarioStep::RestartDesktop => "restart_desktop",
    }
}

fn parse_game_status(value: &str) -> Result<GameRoomStatus> {
    match value {
        "Open" => Ok(GameRoomStatus::Open),
        "InProgress" => Ok(GameRoomStatus::InProgress),
        "Finished" => Ok(GameRoomStatus::Finished),
        _ => anyhow::bail!("unsupported game room status: {value}"),
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

    #[tokio::test]
    async fn desktop_smoke_live_session_persist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root");
        let artifacts = root
            .join("test-results")
            .join("kukuri")
            .join("desktop-smoke-live-session");
        let result = run_named_scenario(root, "desktop_smoke_live_session_persist", &artifacts)
            .await
            .expect("scenario");

        assert_eq!(result.status, HarnessStatus::Pass);
        assert!(artifacts.join("result.json").exists());
    }

    #[tokio::test]
    async fn desktop_smoke_game_room_persist() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root");
        let artifacts = root
            .join("test-results")
            .join("kukuri")
            .join("desktop-smoke-game-room");
        let result = run_named_scenario(root, "desktop_smoke_game_room_persist", &artifacts)
            .await
            .expect("scenario");

        assert_eq!(result.status, HarnessStatus::Pass);
        assert!(artifacts.join("result.json").exists());
    }
}
