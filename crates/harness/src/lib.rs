use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use kukuri_app_api::{
    AppService, CreateGameRoomInput, CreateLiveSessionInput, GameScoreView, SyncStatus,
    UpdateGameRoomInput,
};
use kukuri_cn_core::{JwtConfig, TestDatabase};
use kukuri_cn_iroh_relay::{IrohRelayConfig, SpawnedIrohRelay};
use kukuri_cn_user_api::{
    UserApiConfig, app_router as user_api_app_router, build_state as build_user_api_state,
};
use kukuri_core::GameRoomStatus;
use kukuri_desktop_runtime::{
    AcceptCommunityNodeConsentsRequest, CommunityNodeTargetRequest, CreatePostRequest,
    DesktopRuntime, ImportPeerTicketRequest, ListGameRoomsRequest, ListLiveSessionsRequest,
    ListThreadRequest, ListTimelineRequest, SetCommunityNodeConfigRequest,
};
use kukuri_store::SqliteStore;
use kukuri_transport::{ConnectMode, FakeNetwork, FakeTransport, TransportNetworkConfig};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, timeout};

const DEFAULT_CN_ADMIN_DATABASE_URL: &str = "postgres://cn:cn_password@127.0.0.1:55432/cn";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioKind {
    #[default]
    DesktopSmoke,
    CommunityNodePublicConnectivity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub name: String,
    #[serde(default)]
    pub kind: ScenarioKind,
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

struct CommunityNodeStack {
    database: TestDatabase,
    user_api_task: tokio::task::JoinHandle<()>,
    _iroh_relay: SpawnedIrohRelay,
    base_url: String,
    iroh_relay_url: String,
}

impl CommunityNodeStack {
    async fn spawn(prefix: &str) -> Result<Self> {
        let admin_database_url = community_node_admin_database_url();
        let database = TestDatabase::create(admin_database_url.as_str(), prefix).await?;
        let iroh_relay = kukuri_cn_iroh_relay::spawn_server(IrohRelayConfig {
            http_bind_addr: "127.0.0.1:0"
                .parse()
                .expect("valid loopback relay bind addr"),
            tls: None,
        })
        .await?;
        let iroh_relay_url = format!("http://{}", iroh_relay.http_addr());

        let user_api_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .context("failed to bind community-node user-api listener")?;
        let user_api_addr = user_api_listener.local_addr()?;
        let base_url = format!("http://{user_api_addr}");

        let user_api_state = build_user_api_state(&UserApiConfig {
            bind_addr: user_api_addr,
            database_url: database.database_url.clone(),
            base_url: base_url.clone(),
            public_base_url: base_url.clone(),
            connectivity_urls: vec![iroh_relay_url.clone()],
            jwt_config: JwtConfig::new("kukuri-cn-harness", "test-secret", 3600),
        })
        .await
        .context("failed to build community-node user-api state")?;
        let user_api_task = tokio::spawn(async move {
            axum::serve(
                user_api_listener,
                user_api_app_router(user_api_state)
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("community-node user-api server");
        });

        Ok(Self {
            database,
            user_api_task,
            _iroh_relay: iroh_relay,
            base_url,
            iroh_relay_url,
        })
    }

    async fn shutdown(self) -> Result<()> {
        self.user_api_task.abort();
        self.database.cleanup().await
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

    match scenario.kind {
        ScenarioKind::DesktopSmoke => {
            run_desktop_smoke_scenario(root, scenario, artifacts_dir).await
        }
        ScenarioKind::CommunityNodePublicConnectivity => {
            run_community_node_public_connectivity(scenario, artifacts_dir).await
        }
    }
}

async fn run_desktop_smoke_scenario(
    root: &Path,
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
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

        write_result_artifact(root, artifacts_dir, &result)?;
        Ok::<HarnessResult, anyhow::Error>(result)
    })
    .await
    .context("scenario exceeded overall timeout")?
}

async fn run_community_node_public_connectivity(
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    unsafe { std::env::set_var("KUKURI_DISABLE_KEYRING", "1") };

    let step_timeout = Duration::from_millis(scenario.timeouts.step_ms);
    let overall_timeout = Duration::from_millis(scenario.timeouts.overall_ms);
    let stack = CommunityNodeStack::spawn("community_node_public_connectivity").await?;

    let scenario_result = timeout(overall_timeout, async {
        cleanup_runtime_artifacts(&artifacts_dir.join("cn-desktop-a.db"))?;
        cleanup_runtime_artifacts(&artifacts_dir.join("cn-desktop-b.db"))?;

        let db_a = artifacts_dir.join("cn-desktop-a.db");
        let db_b = artifacts_dir.join("cn-desktop-b.db");
        let mut steps = Vec::new();

        let started_at = Instant::now();
        let runtime_a = DesktopRuntime::new_with_config(&db_a, TransportNetworkConfig::loopback())
            .await
            .context("failed to launch community-node desktop a")?;
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .context("failed to launch community-node desktop b")?;
        push_named_step(&mut steps, "launch_desktops", started_at);

        let started_at = Instant::now();
        runtime_a
            .set_community_node_config(SetCommunityNodeConfigRequest {
                base_urls: vec![stack.base_url.clone()],
            })
            .await
            .context("failed to configure community node for desktop a")?;
        runtime_b
            .set_community_node_config(SetCommunityNodeConfigRequest {
                base_urls: vec![stack.base_url.clone()],
            })
            .await
            .context("failed to configure community node for desktop b")?;
        push_named_step(&mut steps, "configure_community_node", started_at);

        let started_at = Instant::now();
        let authenticated_a = runtime_a
            .authenticate_community_node(CommunityNodeTargetRequest {
                base_url: stack.base_url.clone(),
            })
            .await
            .context("failed to authenticate desktop a with community node")?;
        let authenticated_b = runtime_b
            .authenticate_community_node(CommunityNodeTargetRequest {
                base_url: stack.base_url.clone(),
            })
            .await
            .context("failed to authenticate desktop b with community node")?;
        assert!(authenticated_a.auth_state.authenticated);
        assert!(authenticated_b.auth_state.authenticated);
        assert!(authenticated_a.resolved_urls.is_none());
        assert!(authenticated_b.resolved_urls.is_none());
        assert!(
            !authenticated_a
                .consent_state
                .as_ref()
                .expect("consent state for desktop a")
                .all_required_accepted
        );
        assert!(
            !authenticated_b
                .consent_state
                .as_ref()
                .expect("consent state for desktop b")
                .all_required_accepted
        );
        push_named_step(&mut steps, "authenticate", started_at);

        let started_at = Instant::now();
        let accepted_a = runtime_a
            .accept_community_node_consents(AcceptCommunityNodeConsentsRequest {
                base_url: stack.base_url.clone(),
                policy_slugs: Vec::new(),
            })
            .await
            .context("failed to accept community node consents for desktop a")?;
        let accepted_b = runtime_b
            .accept_community_node_consents(AcceptCommunityNodeConsentsRequest {
                base_url: stack.base_url.clone(),
                policy_slugs: Vec::new(),
            })
            .await
            .context("failed to accept community node consents for desktop b")?;
        assert!(
            accepted_a
                .consent_state
                .as_ref()
                .expect("accepted consent state a")
                .all_required_accepted
        );
        assert!(
            accepted_b
                .consent_state
                .as_ref()
                .expect("accepted consent state b")
                .all_required_accepted
        );
        assert_eq!(
            accepted_a
                .resolved_urls
                .as_ref()
                .expect("resolved urls a")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        assert_eq!(
            accepted_b
                .resolved_urls
                .as_ref()
                .expect("resolved urls b")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        assert!(accepted_a.restart_required);
        assert!(accepted_b.restart_required);
        push_named_step(&mut steps, "accept_consents", started_at);

        let started_at = Instant::now();
        runtime_a.shutdown().await;
        runtime_b.shutdown().await;
        let runtime_a = DesktopRuntime::new_with_config(&db_a, TransportNetworkConfig::loopback())
            .await
            .context("failed to restart community-node desktop a")?;
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .context("failed to restart community-node desktop b")?;
        let restarted_a = runtime_a
            .get_community_node_statuses()
            .await
            .context("failed to load community-node status for desktop a after restart")?
            .into_iter()
            .next()
            .context("missing community-node status for desktop a after restart")?;
        let restarted_b = runtime_b
            .get_community_node_statuses()
            .await
            .context("failed to load community-node status for desktop b after restart")?
            .into_iter()
            .next()
            .context("missing community-node status for desktop b after restart")?;
        assert!(restarted_a.auth_state.authenticated);
        assert!(restarted_b.auth_state.authenticated);
        assert!(!restarted_a.restart_required);
        assert!(!restarted_b.restart_required);
        assert_eq!(
            restarted_a
                .resolved_urls
                .as_ref()
                .expect("resolved urls a after restart")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        assert_eq!(
            restarted_b
                .resolved_urls
                .as_ref()
                .expect("resolved urls b after restart")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        let sync_a = runtime_a
            .get_sync_status()
            .await
            .context("failed to load sync status for desktop a after restart")?;
        let sync_b = runtime_b
            .get_sync_status()
            .await
            .context("failed to load sync status for desktop b after restart")?;
        assert_eq!(sync_a.discovery.connect_mode, ConnectMode::DirectOrRelay);
        assert_eq!(sync_b.discovery.connect_mode, ConnectMode::DirectOrRelay);
        push_named_step(&mut steps, "restart_desktops", started_at);

        let topic = scenario.fixtures.topic.as_str();
        let started_at = Instant::now();
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await?
            .context("desktop a peer ticket is unavailable")?;
        let ticket_b = runtime_b
            .local_peer_ticket()
            .await?
            .context("desktop b peer ticket is unavailable")?;
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_b })
            .await
            .context("failed to import desktop b ticket into desktop a")?;
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .context("failed to import desktop a ticket into desktop b")?;
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop a to scenario topic")?;
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop b to scenario topic")?;
        wait_for_topic_peer_count(&runtime_a, topic, 1, step_timeout)
            .await
            .context("desktop a did not observe initial topic peer connectivity")?;
        wait_for_topic_peer_count(&runtime_b, topic, 1, step_timeout)
            .await
            .context("desktop b did not observe initial topic peer connectivity")?;
        push_named_step(&mut steps, "connect_desktops", started_at);

        let started_at = Instant::now();
        let post_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "community node scenario post".to_string(),
                reply_to: None,
                attachments: Vec::new(),
            })
            .await
            .context("failed to create scenario post on desktop a")?;
        wait_for_timeline_object(&runtime_b, topic, post_id.as_str(), step_timeout)
            .await
            .context("desktop b did not receive the initial scenario post")?;
        push_named_step(&mut steps, "post", started_at);

        let started_at = Instant::now();
        let reply_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "community node scenario reply".to_string(),
                reply_to: Some(post_id.clone()),
                attachments: Vec::new(),
            })
            .await
            .context("failed to create scenario reply on desktop b")?;
        wait_for_thread_object(
            &runtime_a,
            topic,
            post_id.as_str(),
            reply_id.as_str(),
            step_timeout,
        )
        .await?;
        push_named_step(&mut steps, "reply_thread", started_at);

        let started_at = Instant::now();
        let session_id = runtime_a
            .create_live_session(kukuri_desktop_runtime::CreateLiveSessionRequest {
                topic: topic.to_string(),
                title: "community live".to_string(),
                description: "live session".to_string(),
            })
            .await
            .context("failed to create live session on desktop a")?;
        wait_for_live_session(&runtime_b, topic, session_id.as_str(), step_timeout).await?;
        runtime_b
            .join_live_session(kukuri_desktop_runtime::LiveSessionCommandRequest {
                topic: topic.to_string(),
                session_id: session_id.clone(),
            })
            .await
            .context("failed to join live session on desktop b")?;
        wait_for_live_viewer_count(&runtime_a, topic, session_id.as_str(), 1, step_timeout).await?;
        runtime_a
            .end_live_session(kukuri_desktop_runtime::LiveSessionCommandRequest {
                topic: topic.to_string(),
                session_id: session_id.clone(),
            })
            .await
            .context("failed to end live session on desktop a")?;
        wait_for_live_ended(&runtime_b, topic, session_id.as_str(), step_timeout).await?;
        push_named_step(&mut steps, "live", started_at);

        let started_at = Instant::now();
        let room_id = runtime_a
            .create_game_room(kukuri_desktop_runtime::CreateGameRoomRequest {
                topic: topic.to_string(),
                title: "community finals".to_string(),
                description: "set".to_string(),
                participants: vec!["Alice".to_string(), "Bob".to_string()],
            })
            .await
            .context("failed to create game room on desktop a")?;
        let room_a = wait_for_game_room(&runtime_a, topic, room_id.as_str(), step_timeout).await?;
        let _room_b = wait_for_game_room(&runtime_b, topic, room_id.as_str(), step_timeout).await?;
        let scores = room_a
            .scores
            .iter()
            .map(|entry| {
                let score = match entry.label.as_str() {
                    "Alice" => 2,
                    "Bob" => 1,
                    _ => entry.score,
                };
                GameScoreView {
                    participant_id: entry.participant_id.clone(),
                    label: entry.label.clone(),
                    score,
                }
            })
            .collect();
        runtime_a
            .update_game_room(kukuri_desktop_runtime::UpdateGameRoomRequest {
                topic: topic.to_string(),
                room_id: room_id.clone(),
                status: GameRoomStatus::Running,
                phase_label: Some("Round 1".to_string()),
                scores,
            })
            .await
            .context("failed to update game room on desktop a")?;
        wait_for_game_score(
            &runtime_b,
            topic,
            room_id.as_str(),
            "Alice",
            2,
            step_timeout,
        )
        .await?;
        push_named_step(&mut steps, "game", started_at);

        let started_at = Instant::now();
        runtime_b.shutdown().await;
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .context("failed to restart community-node desktop b for reconnect")?;
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .context("failed to read desktop a peer ticket for reconnect")?
            .context("desktop a peer ticket is unavailable for reconnect")?;
        let ticket_b = runtime_b
            .local_peer_ticket()
            .await
            .context("failed to read desktop b peer ticket after reconnect restart")?
            .context("desktop b peer ticket is unavailable after reconnect restart")?;
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_b })
            .await
            .context("failed to reimport desktop b ticket into desktop a after restart")?;
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest { ticket: ticket_a })
            .await
            .context("failed to reimport desktop a ticket into desktop b after restart")?;
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to resubscribe desktop b to scenario topic after reconnect")?;
        let reconnect_post = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "community node reconnect".to_string(),
                reply_to: None,
                attachments: Vec::new(),
            })
            .await
            .context("failed to create reconnect post on desktop a")?;
        wait_for_timeline_object(&runtime_b, topic, reconnect_post.as_str(), step_timeout)
            .await
            .context("desktop b did not receive the reconnect post after restart")?;
        let metrics_snapshot = if scenario.artifacts.metrics_snapshot {
            Some(
                runtime_b
                    .get_sync_status()
                    .await
                    .context("failed to collect final sync status for desktop b")?,
            )
        } else {
            None
        };
        runtime_a.shutdown().await;
        runtime_b.shutdown().await;
        push_named_step(&mut steps, "reconnect", started_at);

        let result = HarnessResult {
            status: HarnessStatus::Pass,
            scenario: scenario.name.clone(),
            steps,
            artifacts: vec![artifacts_dir.join("result.json").display().to_string()],
            metrics_snapshot,
        };
        write_result_artifact(Path::new("."), artifacts_dir, &result)?;
        Ok::<HarnessResult, anyhow::Error>(result)
    })
    .await
    .context("scenario exceeded overall timeout")
    .and_then(|result| result);

    let shutdown_result = stack.shutdown().await;
    match (scenario_result, shutdown_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(scenario_error), Err(shutdown_error)) => Err(scenario_error.context(format!(
            "failed to tear down community-node stack after scenario error: {shutdown_error:#}"
        ))),
    }
}

fn write_result_artifact(_root: &Path, artifacts_dir: &Path, result: &HarnessResult) -> Result<()> {
    let payload = serde_json::to_string_pretty(result)?;
    std::fs::write(artifacts_dir.join("result.json"), payload).with_context(|| {
        format!(
            "failed to write result artifact under {}",
            artifacts_dir.display()
        )
    })
}

fn community_node_admin_database_url() -> String {
    std::env::var("COMMUNITY_NODE_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_CN_ADMIN_DATABASE_URL.to_string())
}

fn cleanup_runtime_artifacts(db_path: &Path) -> Result<()> {
    let config_paths = [
        db_path.to_path_buf(),
        db_path.with_extension("iroh-data"),
        db_path.with_extension("community-node.json"),
        db_path.with_extension("identity-store"),
        db_path.with_extension("nsec"),
    ];
    for path in config_paths {
        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove stale directory {}", path.display()))?;
        } else if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove stale file {}", path.display()))?;
        }
    }
    Ok(())
}

fn push_named_step(steps: &mut Vec<StepResult>, action: &str, started_at: Instant) {
    steps.push(StepResult {
        action: action.to_string(),
        duration_ms: started_at.elapsed().as_millis(),
    });
}

async fn wait_for_timeline_object(
    runtime: &DesktopRuntime,
    topic: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    cursor: None,
                    limit: Some(50),
                })
                .await?;
            if timeline
                .items
                .iter()
                .any(|item| item.object_id == object_id)
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("timeline assertion timeout")?
}

async fn wait_for_thread_object(
    runtime: &DesktopRuntime,
    topic: &str,
    thread_id: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let thread = runtime
                .list_thread(ListThreadRequest {
                    topic: topic.to_string(),
                    thread_id: thread_id.to_string(),
                    cursor: None,
                    limit: Some(50),
                })
                .await?;
            if thread.items.iter().any(|item| item.object_id == object_id) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("thread assertion timeout")?
}

async fn wait_for_topic_peer_count(
    runtime: &DesktopRuntime,
    topic: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let status = runtime.get_sync_status().await?;
            if status
                .topic_diagnostics
                .iter()
                .any(|entry| entry.topic == topic && entry.peer_count >= expected)
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("topic peer-count assertion timeout")?
}

async fn wait_for_live_session(
    runtime: &DesktopRuntime,
    topic: &str,
    session_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                })
                .await?;
            if sessions
                .iter()
                .any(|session| session.session_id == session_id)
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("live-session assertion timeout")?
}

async fn wait_for_live_viewer_count(
    runtime: &DesktopRuntime,
    topic: &str,
    session_id: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                })
                .await?;
            if sessions
                .iter()
                .any(|session| session.session_id == session_id && session.viewer_count == expected)
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("live-session viewer assertion timeout")?
}

async fn wait_for_live_ended(
    runtime: &DesktopRuntime,
    topic: &str,
    session_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                })
                .await?;
            if sessions.iter().any(|session| {
                session.session_id == session_id
                    && session.status == kukuri_core::LiveSessionStatus::Ended
            }) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("live-session ended assertion timeout")?
}

async fn wait_for_game_room(
    runtime: &DesktopRuntime,
    topic: &str,
    room_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::GameRoomView> {
    timeout(step_timeout, async {
        loop {
            let rooms = runtime
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.to_string(),
                })
                .await?;
            if let Some(room) = rooms.into_iter().find(|room| room.room_id == room_id) {
                return Ok::<kukuri_app_api::GameRoomView, anyhow::Error>(room);
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("game-room assertion timeout")?
}

async fn wait_for_game_score(
    runtime: &DesktopRuntime,
    topic: &str,
    room_id: &str,
    label: &str,
    expected: i64,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let rooms = runtime
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.to_string(),
                })
                .await?;
            if rooms.iter().any(|room| {
                room.room_id == room_id
                    && room
                        .scores
                        .iter()
                        .any(|score| score.label == label && score.score == expected)
            }) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("game-score assertion timeout")?
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
        "Open" | "Waiting" => Ok(GameRoomStatus::Waiting),
        "InProgress" | "Running" => Ok(GameRoomStatus::Running),
        "Paused" => Ok(GameRoomStatus::Paused),
        "Finished" | "Ended" => Ok(GameRoomStatus::Ended),
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
