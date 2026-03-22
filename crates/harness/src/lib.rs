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
use kukuri_core::{ChannelRef, GameRoomStatus, KukuriKeys, TimelineScope};
use kukuri_desktop_runtime::{
    AcceptCommunityNodeConsentsRequest, CommunityNodeTargetRequest, CreateGameRoomRequest,
    CreateLiveSessionRequest, CreatePostRequest, CreatePrivateChannelRequest, DesktopRuntime,
    ExportPrivateChannelInviteRequest, ImportPeerTicketRequest, ImportPrivateChannelInviteRequest,
    ListGameRoomsRequest, ListJoinedPrivateChannelsRequest, ListLiveSessionsRequest,
    ListThreadRequest, ListTimelineRequest, LiveSessionCommandRequest,
    SetCommunityNodeConfigRequest,
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
    CommunityNodeMultiDeviceConnectivity,
    PrivateChannelInviteConnectivity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommunityNodeIdentityMode {
    DistinctUsers,
    SharedIdentity,
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
            run_community_node_connectivity(
                scenario,
                artifacts_dir,
                CommunityNodeIdentityMode::DistinctUsers,
            )
            .await
        }
        ScenarioKind::CommunityNodeMultiDeviceConnectivity => {
            run_community_node_connectivity(
                scenario,
                artifacts_dir,
                CommunityNodeIdentityMode::SharedIdentity,
            )
            .await
        }
        ScenarioKind::PrivateChannelInviteConnectivity => {
            run_private_channel_invite_connectivity(scenario, artifacts_dir).await
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

async fn run_community_node_connectivity(
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
    identity_mode: CommunityNodeIdentityMode,
) -> Result<HarnessResult> {
    unsafe { std::env::set_var("KUKURI_DISABLE_KEYRING", "1") };

    let step_timeout = Duration::from_millis(scenario.timeouts.step_ms);
    let overall_timeout = Duration::from_millis(scenario.timeouts.overall_ms);
    let stack = CommunityNodeStack::spawn(match identity_mode {
        CommunityNodeIdentityMode::DistinctUsers => "community_node_public_connectivity",
        CommunityNodeIdentityMode::SharedIdentity => "community_node_multi_device_connectivity",
    })
    .await?;

    let scenario_result = timeout(overall_timeout, async {
        cleanup_runtime_artifacts(&artifacts_dir.join("cn-desktop-a.db"))?;
        cleanup_runtime_artifacts(&artifacts_dir.join("cn-desktop-b.db"))?;

        let db_a = artifacts_dir.join("cn-desktop-a.db");
        let db_b = artifacts_dir.join("cn-desktop-b.db");
        if identity_mode == CommunityNodeIdentityMode::SharedIdentity {
            let shared_keys = KukuriKeys::generate();
            persist_runtime_identity(&db_a, &shared_keys)?;
            persist_runtime_identity(&db_b, &shared_keys)?;
        }
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
        assert!(!accepted_a.restart_required);
        assert!(!accepted_b.restart_required);
        push_named_step(&mut steps, "accept_consents", started_at);

        let started_at = Instant::now();
        let refreshed_a = runtime_a
            .get_community_node_statuses()
            .await
            .context("failed to load community-node status for desktop a after consent")?
            .into_iter()
            .next()
            .context("missing community-node status for desktop a after consent")?;
        let refreshed_b = runtime_b
            .get_community_node_statuses()
            .await
            .context("failed to load community-node status for desktop b after consent")?
            .into_iter()
            .next()
            .context("missing community-node status for desktop b after consent")?;
        assert!(refreshed_a.auth_state.authenticated);
        assert!(refreshed_b.auth_state.authenticated);
        assert!(!refreshed_a.restart_required);
        assert!(!refreshed_b.restart_required);
        assert_eq!(
            refreshed_a
                .resolved_urls
                .as_ref()
                .expect("resolved urls a after consent")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        assert_eq!(
            refreshed_b
                .resolved_urls
                .as_ref()
                .expect("resolved urls b after consent")
                .connectivity_urls,
            vec![stack.iroh_relay_url.clone()]
        );
        let sync_a = runtime_a
            .get_sync_status()
            .await
            .context("failed to load sync status for desktop a after consent")?;
        let sync_b = runtime_b
            .get_sync_status()
            .await
            .context("failed to load sync status for desktop b after consent")?;
        match identity_mode {
            CommunityNodeIdentityMode::DistinctUsers => {
                assert_ne!(sync_a.local_author_pubkey, sync_b.local_author_pubkey);
            }
            CommunityNodeIdentityMode::SharedIdentity => {
                assert_eq!(sync_a.local_author_pubkey, sync_b.local_author_pubkey);
            }
        }
        assert_eq!(sync_a.discovery.connect_mode, ConnectMode::DirectOrRelay);
        assert_eq!(sync_b.discovery.connect_mode, ConnectMode::DirectOrRelay);
        push_named_step(&mut steps, "refresh_connectivity", started_at);

        let topic = scenario.fixtures.topic.as_str();
        let started_at = Instant::now();
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop a to scenario topic")?;
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
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
            .context("desktop b did not observe initial community-node topic peer connectivity")?;
        push_named_step(&mut steps, "community_node_connectivity", started_at);

        let started_at = Instant::now();
        let post_id = replicate_public_post_with_retry(
            &runtime_a,
            &runtime_b,
            topic,
            "community node scenario post",
            step_timeout,
            PublicReplicationLabels {
                failure: "initial scenario post",
                publisher: "desktop a",
                subscriber: "desktop b",
            },
        )
        .await?;
        push_named_step(&mut steps, "post", started_at);

        let started_at = Instant::now();
        let reply_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "community node scenario reply".to_string(),
                reply_to: Some(post_id.clone()),
                channel_ref: ChannelRef::Public,
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

        if identity_mode == CommunityNodeIdentityMode::DistinctUsers {
            let started_at = Instant::now();
            let session_id = runtime_a
                .create_live_session(kukuri_desktop_runtime::CreateLiveSessionRequest {
                    topic: topic.to_string(),
                    channel_ref: ChannelRef::Public,
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
            wait_for_live_viewer_count(&runtime_a, topic, session_id.as_str(), 1, step_timeout)
                .await?;
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
                    channel_ref: ChannelRef::Public,
                    title: "community finals".to_string(),
                    description: "set".to_string(),
                    participants: vec!["Alice".to_string(), "Bob".to_string()],
                })
                .await
                .context("failed to create game room on desktop a")?;
            let room_a =
                wait_for_game_room(&runtime_a, topic, room_id.as_str(), step_timeout).await?;
            let _room_b =
                wait_for_game_room(&runtime_b, topic, room_id.as_str(), step_timeout).await?;
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
        }

        let started_at = Instant::now();
        shutdown_runtime(runtime_b, "desktop b reconnect pre-shutdown")
            .await
            .context("community-node reconnect shutdown timed out")?;
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .context("failed to restart community-node desktop b for reconnect")?;
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: TimelineScope::Public,
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to resubscribe desktop b to scenario topic after reconnect")?;
        let _reconnect_probe_post = replicate_public_post_with_retry(
            &runtime_b,
            &runtime_a,
            topic,
            "community node reconnect probe",
            step_timeout,
            PublicReplicationLabels {
                failure: "reconnect probe post after restart",
                publisher: "desktop b",
                subscriber: "desktop a",
            },
        )
        .await?;
        let _reconnect_post = replicate_public_post_with_retry(
            &runtime_a,
            &runtime_b,
            topic,
            "community node reconnect",
            step_timeout,
            PublicReplicationLabels {
                failure: "reconnect post after restart",
                publisher: "desktop a",
                subscriber: "desktop b",
            },
        )
        .await?;
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
        shutdown_runtime(runtime_a, "desktop a final shutdown")
            .await
            .context("community-node desktop a final shutdown timed out")?;
        shutdown_runtime(runtime_b, "desktop b final shutdown")
            .await
            .context("community-node desktop b final shutdown timed out")?;
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

    let shutdown_result = timeout(Duration::from_secs(30), stack.shutdown())
        .await
        .context("community-node stack shutdown timed out")
        .and_then(|result| result);
    match (scenario_result, shutdown_result) {
        (Ok(result), Ok(())) => Ok(result),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(scenario_error), Err(shutdown_error)) => Err(scenario_error.context(format!(
            "failed to tear down community-node stack after scenario error: {shutdown_error:#}"
        ))),
    }
}

async fn run_private_channel_invite_connectivity(
    scenario: &ScenarioSpec,
    artifacts_dir: &Path,
) -> Result<HarnessResult> {
    unsafe { std::env::set_var("KUKURI_DISABLE_KEYRING", "1") };

    let db_a = artifacts_dir.join("private-channel-a.db");
    let db_b = artifacts_dir.join("private-channel-b.db");
    let db_c = artifacts_dir.join("private-channel-c.db");
    cleanup_runtime_artifacts(&db_a)?;
    cleanup_runtime_artifacts(&db_b)?;
    cleanup_runtime_artifacts(&db_c)?;

    let runtime_a = DesktopRuntime::new_with_config(&db_a, TransportNetworkConfig::loopback())
        .await
        .context("failed to launch desktop a for private-channel scenario")?;
    let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
        .await
        .context("failed to launch desktop b for private-channel scenario")?;
    let runtime_c = DesktopRuntime::new_with_config(&db_c, TransportNetworkConfig::loopback())
        .await
        .context("failed to launch desktop c for private-channel scenario")?;
    let overall_timeout =
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_millis(scenario.timeouts.overall_ms).max(Duration::from_secs(600))
        } else {
            Duration::from_millis(scenario.timeouts.overall_ms)
        };
    let step_timeout =
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_millis(scenario.timeouts.step_ms).max(Duration::from_secs(180))
        } else {
            Duration::from_millis(scenario.timeouts.step_ms)
        };

    timeout(overall_timeout, async move {
        let mut steps = Vec::new();
        let topic = scenario.fixtures.topic.as_str();
        let mut runtime_b = runtime_b;

        let started_at = Instant::now();
        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .context("failed to export ticket for desktop a")?
            .context("missing ticket for desktop a")?;
        let ticket_b = runtime_b
            .local_peer_ticket()
            .await
            .context("failed to export ticket for desktop b")?
            .context("missing ticket for desktop b")?;
        let ticket_c = runtime_c
            .local_peer_ticket()
            .await
            .context("failed to export ticket for desktop c")?
            .context("missing ticket for desktop c")?;
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .context("failed to import desktop b ticket into desktop a")?;
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .context("failed to import desktop a ticket into desktop b")?;
        push_named_step(&mut steps, "connect", started_at);

        let started_at = Instant::now();
        let public_scope = TimelineScope::Public;
        let all_joined_scope = TimelineScope::AllJoined;
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop a to public topic")?;
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop b to public topic")?;
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .context("failed to rebuild desktop b ticket into desktop a after subscribe")?;
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .context("failed to rebuild desktop a ticket into desktop b after subscribe")?;
        wait_for_topic_peer_count(&runtime_a, topic, 1, step_timeout)
            .await
            .context("desktop a did not observe public topic connectivity")?;
        wait_for_topic_peer_count(&runtime_b, topic, 1, step_timeout)
            .await
            .context("desktop b did not observe public topic connectivity")?;
        push_named_step(&mut steps, "public_sync", started_at);

        let started_at = Instant::now();
        let channel = runtime_a
            .create_private_channel(CreatePrivateChannelRequest {
                topic: topic.to_string(),
                label: "core".to_string(),
                audience_kind: kukuri_core::ChannelAudienceKind::InviteOnly,
            })
            .await
            .context("failed to create private channel")?;
        push_named_step(&mut steps, "create_channel", started_at);

        let started_at = Instant::now();
        let invite = runtime_a
            .export_private_channel_invite(ExportPrivateChannelInviteRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .context("failed to export private channel invite")?;
        push_named_step(&mut steps, "create_invite", started_at);

        let started_at = Instant::now();
        let preview = runtime_b
            .import_private_channel_invite(ImportPrivateChannelInviteRequest { token: invite })
            .await
            .context("failed to import private channel invite")?;
        assert_eq!(preview.topic_id.as_str(), topic);
        assert_eq!(preview.channel_id.as_str(), channel.channel_id);
        wait_for_joined_private_channel(
            &runtime_b,
            topic,
            channel.channel_id.as_str(),
            step_timeout,
        )
        .await
        .context("desktop b did not join private channel after invite import")?;
        let joined_channels = runtime_b
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.to_string(),
            })
            .await
            .context("failed to list joined private channels after invite import")?;
        assert!(
            joined_channels
                .iter()
                .any(|entry| entry.channel_id == channel.channel_id && entry.label == "core")
        );
        push_named_step(&mut steps, "import_invite", started_at);

        let private_channel_id = kukuri_core::ChannelId::new(channel.channel_id.clone());
        let private_scope = TimelineScope::Channel {
            channel_id: private_channel_id.clone(),
        };
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: private_channel_id.clone(),
        };
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop a to private channel")?;
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop b to private channel")?;
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .context("failed to refresh desktop b ticket into desktop a for private channel")?;
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .context("failed to refresh desktop a ticket into desktop b for private channel")?;
        let started_at = Instant::now();
        let private_post_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "private post".to_string(),
                reply_to: None,
                channel_ref: private_ref.clone(),
                attachments: Vec::new(),
            })
            .await
            .context("failed to create private post")?;
        let private_post_attempts =
            if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
                3
            } else {
                1
            };
        let private_post_timeout = if private_post_attempts > 1 {
            Duration::from_millis(
                (step_timeout.as_millis() / private_post_attempts as u128)
                    .max(1)
                    .try_into()
                    .expect("private post timeout fits in u64"),
            )
        } else {
            step_timeout
        };
        let mut private_post_error = None;
        for attempt in 1..=private_post_attempts {
            match wait_for_timeline_object_in_scope(
                &runtime_a,
                topic,
                private_scope.clone(),
                private_post_id.as_str(),
                private_post_timeout,
            )
            .await
            {
                Ok(_) => {
                    private_post_error = None;
                    break;
                }
                Err(error) if attempt < private_post_attempts => {
                    private_post_error = Some(format!("{error:#}"));
                    runtime_a
                        .import_peer_ticket(ImportPeerTicketRequest {
                            ticket: ticket_b.clone(),
                        })
                        .await
                        .context("failed to refresh desktop b ticket into desktop a after private post timeout")?;
                    runtime_b
                        .import_peer_ticket(ImportPeerTicketRequest {
                            ticket: ticket_a.clone(),
                        })
                        .await
                        .context("failed to refresh desktop a ticket into desktop b after private post timeout")?;
                    let _ = runtime_a
                        .list_timeline(ListTimelineRequest {
                            topic: topic.to_string(),
                            scope: private_scope.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    let _ = runtime_b
                        .list_timeline(ListTimelineRequest {
                            topic: topic.to_string(),
                            scope: private_scope.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    private_post_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }
        if let Some(error) = private_post_error {
            let status_a = runtime_a
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read desktop a sync status".to_string());
            let status_b = runtime_b
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read desktop b sync status".to_string());
            let joined_a = runtime_a
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.to_string(),
                })
                .await
                .unwrap_or_default();
            let joined_b = runtime_b
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.to_string(),
                })
                .await
                .unwrap_or_default();
            return Err(anyhow::anyhow!(error).context(format!(
                "desktop a did not receive private post; desktop_a=({status_a}); desktop_b=({status_b}); joined_a={joined_a:?}; joined_b={joined_b:?}"
            )));
        }
        assert_timeline_scope_excludes_object(
            &runtime_b,
            topic,
            public_scope.clone(),
            private_post_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .context("desktop b public scope leaked private post")?;
        push_named_step(&mut steps, "private_post", started_at);

        let started_at = Instant::now();
        let private_reply_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "private reply".to_string(),
                reply_to: Some(private_post_id.clone()),
                channel_ref: ChannelRef::Public,
                attachments: Vec::new(),
            })
            .await
            .context("failed to create private reply")?;
        let private_reply_attempts =
            if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
                3
            } else {
                1
            };
        let private_reply_timeout = if private_reply_attempts > 1 {
            Duration::from_millis(
                (step_timeout.as_millis() / private_reply_attempts as u128)
                    .max(1)
                    .try_into()
                    .expect("private reply timeout fits in u64"),
            )
        } else {
            step_timeout
        };
        let mut private_reply_error = None;
        for attempt in 1..=private_reply_attempts {
            match wait_for_thread_object(
                &runtime_a,
                topic,
                private_post_id.as_str(),
                private_reply_id.as_str(),
                private_reply_timeout,
            )
            .await
            {
                Ok(_) => {
                    private_reply_error = None;
                    break;
                }
                Err(error) if attempt < private_reply_attempts => {
                    private_reply_error = Some(format!("{error:#}"));
                    runtime_a
                        .import_peer_ticket(ImportPeerTicketRequest {
                            ticket: ticket_b.clone(),
                        })
                        .await
                        .context("failed to refresh desktop b ticket into desktop a after private reply timeout")?;
                    runtime_b
                        .import_peer_ticket(ImportPeerTicketRequest {
                            ticket: ticket_a.clone(),
                        })
                        .await
                        .context("failed to refresh desktop a ticket into desktop b after private reply timeout")?;
                    let _ = runtime_a
                        .list_timeline(ListTimelineRequest {
                            topic: topic.to_string(),
                            scope: private_scope.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    let _ = runtime_b
                        .list_timeline(ListTimelineRequest {
                            topic: topic.to_string(),
                            scope: private_scope.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    let _ = runtime_a
                        .list_thread(ListThreadRequest {
                            topic: topic.to_string(),
                            thread_id: private_post_id.clone(),
                            cursor: None,
                            limit: Some(20),
                        })
                        .await;
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    private_reply_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }
        if let Some(error) = private_reply_error {
            anyhow::bail!("desktop a did not receive private reply in thread: {error}");
        }
        let private_thread = runtime_a
            .list_thread(ListThreadRequest {
                topic: topic.to_string(),
                thread_id: private_post_id.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to read private thread on desktop a")?;
        assert!(private_thread.items.iter().any(|post| {
            post.object_id == private_reply_id
                && post.channel_id.as_deref() == Some(channel.channel_id.as_str())
        }));
        push_named_step(&mut steps, "private_reply_thread", started_at);

        let (private_replication_attempts, private_replication_timeout) =
            private_replication_retry_schedule(step_timeout);

        let started_at = Instant::now();
        let session_id = runtime_b
            .create_live_session(CreateLiveSessionRequest {
                topic: topic.to_string(),
                channel_ref: private_ref.clone(),
                title: "private live".to_string(),
                description: "core stream".to_string(),
            })
            .await
            .context("failed to create private live session")?;
        let mut live_session_error = None;
        for attempt in 1..=private_replication_attempts {
            match wait_for_live_session_in_scope(
                &runtime_a,
                topic,
                private_scope.clone(),
                session_id.as_str(),
                private_replication_timeout,
            )
            .await
            {
                Ok(_) => {
                    live_session_error = None;
                    break;
                }
                Err(error) if attempt < private_replication_attempts => {
                    live_session_error = Some(format!("{error:#}"));
                    refresh_private_channel_pair(
                        &runtime_a,
                        &runtime_b,
                        ticket_a.as_str(),
                        ticket_b.as_str(),
                        topic,
                        &private_scope,
                    )
                    .await
                    .context("failed to refresh private channel after live-session timeout")?;
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    live_session_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }
        if let Some(error) = live_session_error {
            anyhow::bail!("desktop a did not receive private live session: {error}");
        }
        runtime_b
            .end_live_session(LiveSessionCommandRequest {
                topic: topic.to_string(),
                session_id: session_id.clone(),
            })
            .await
            .context("failed to end private live session on desktop b")?;
        let mut live_ended_error = None;
        for attempt in 1..=private_replication_attempts {
            match wait_for_live_ended_in_scope(
                &runtime_a,
                topic,
                private_scope.clone(),
                session_id.as_str(),
                private_replication_timeout,
            )
            .await
            {
                Ok(_) => {
                    live_ended_error = None;
                    break;
                }
                Err(error) if attempt < private_replication_attempts => {
                    live_ended_error = Some(format!("{error:#}"));
                    refresh_private_channel_pair(
                        &runtime_a,
                        &runtime_b,
                        ticket_a.as_str(),
                        ticket_b.as_str(),
                        topic,
                        &private_scope,
                    )
                    .await
                    .context("failed to refresh private channel after live-ended timeout")?;
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    live_ended_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }
        if let Some(error) = live_ended_error {
            anyhow::bail!("desktop a did not observe ended private live session: {error}");
        }
        push_named_step(&mut steps, "private_live", started_at);

        let started_at = Instant::now();
        let room_id = runtime_b
            .create_game_room(CreateGameRoomRequest {
                topic: topic.to_string(),
                channel_ref: private_ref.clone(),
                title: "private finals".to_string(),
                description: "core bracket".to_string(),
                participants: vec!["Alice".to_string(), "Bob".to_string()],
            })
            .await
            .context("failed to create private game room")?;
        let mut room_a = None;
        let mut game_room_error = None;
        for attempt in 1..=private_replication_attempts {
            match wait_for_game_room_in_scope(
                &runtime_a,
                topic,
                private_scope.clone(),
                room_id.as_str(),
                private_replication_timeout,
            )
            .await
            {
                Ok(room) => {
                    room_a = Some(room);
                    game_room_error = None;
                    break;
                }
                Err(error) if attempt < private_replication_attempts => {
                    game_room_error = Some(format!("{error:#}"));
                    refresh_private_channel_pair(
                        &runtime_a,
                        &runtime_b,
                        ticket_a.as_str(),
                        ticket_b.as_str(),
                        topic,
                        &private_scope,
                    )
                    .await
                    .context("failed to refresh private channel after game-room timeout")?;
                    sleep(Duration::from_millis(250)).await;
                }
                Err(error) => {
                    game_room_error = Some(format!("{error:#}"));
                    break;
                }
            }
        }
        if let Some(error) = game_room_error {
            anyhow::bail!("desktop a did not receive private game room: {error}");
        }
        let room_a = room_a.expect("private game room should be available after successful wait");
        assert_eq!(room_a.title, "private finals");
        push_named_step(&mut steps, "private_game", started_at);

        let started_at = Instant::now();
        shutdown_runtime(runtime_b, "desktop b private-channel restart pre-shutdown")
            .await
            .context("failed to shut down desktop b before restart")?;
        remove_sqlite_runtime_db(&db_b)
            .with_context(|| format!("failed to remove {} before restart", db_b.display()))?;
        runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .context("failed to restart desktop b for private-channel scenario")?;
        let joined_after_restart = runtime_b
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.to_string(),
            })
            .await
            .context("failed to list joined private channels after restart")?;
        assert!(
            joined_after_restart
                .iter()
                .any(|entry| entry.channel_id == channel.channel_id && entry.label == "core")
        );
        wait_for_timeline_object_in_scope(
            &runtime_b,
            topic,
            private_scope.clone(),
            private_post_id.as_str(),
            step_timeout,
        )
        .await
        .context("desktop b did not restore private post after restart")?;
        let private_thread_after_restart = runtime_b
            .list_thread(ListThreadRequest {
                topic: topic.to_string(),
                thread_id: private_post_id.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to read private thread after restart")?;
        assert!(
            private_thread_after_restart
                .items
                .iter()
                .any(|post| post.object_id == private_reply_id)
        );
        wait_for_live_ended_in_scope(
            &runtime_b,
            topic,
            private_scope.clone(),
            session_id.as_str(),
            step_timeout,
        )
        .await
        .context("desktop b did not restore private live session after restart")?;
        let restored_room = wait_for_game_room_in_scope(
            &runtime_b,
            topic,
            private_scope.clone(),
            room_id.as_str(),
            step_timeout,
        )
        .await
        .context("desktop b did not restore private game room after restart")?;
        assert_eq!(restored_room.title, "private finals");
        let fresh_invite = runtime_b
            .export_private_channel_invite(ExportPrivateChannelInviteRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .context("failed to re-export private invite after restart")?;
        assert!(fresh_invite.contains(topic));
        assert!(fresh_invite.contains(channel.channel_id.as_str()));
        push_named_step(&mut steps, "restart_rehydrate", started_at);

        let started_at = Instant::now();
        let ticket_b_after_restart = runtime_b
            .local_peer_ticket()
            .await
            .context("failed to export restarted desktop b ticket")?
            .context("missing restarted desktop b ticket")?;
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_c.clone(),
            })
            .await
            .context("failed to import desktop c ticket into desktop a for outsider check")?;
        runtime_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .context("failed to import desktop a ticket into desktop c for outsider check")?;
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_c.clone(),
            })
            .await
            .context("failed to import desktop c ticket into desktop b for outsider check")?;
        runtime_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b_after_restart,
            })
            .await
            .context("failed to import restarted desktop b ticket into desktop c")?;
        let _ = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop c to public topic")?;
        let _ = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: all_joined_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .context("failed to subscribe desktop c to all-joined topic")?;
        wait_for_topic_peer_count(&runtime_c, topic, 1, step_timeout)
            .await
            .context("desktop c did not connect as outsider")?;
        let joined_channels_c = runtime_c
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.to_string(),
            })
            .await
            .context("failed to list desktop c joined private channels")?;
        assert!(
            joined_channels_c
                .iter()
                .all(|entry| entry.channel_id != channel.channel_id)
        );
        assert_timeline_scope_excludes_object(
            &runtime_c,
            topic,
            public_scope.clone(),
            private_post_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .context("desktop c public scope leaked private post")?;
        assert_timeline_scope_excludes_object(
            &runtime_c,
            topic,
            all_joined_scope.clone(),
            private_post_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .context("desktop c all-joined scope leaked private post")?;
        assert_timeline_scope_excludes_object(
            &runtime_c,
            topic,
            all_joined_scope.clone(),
            private_reply_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .context("desktop c all-joined scope leaked private reply")?;
        assert_live_session_absent_in_scope(
            &runtime_c,
            topic,
            all_joined_scope.clone(),
            session_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .context("desktop c leaked private live session")?;
        assert_game_room_absent_in_scope(
            &runtime_c,
            topic,
            all_joined_scope.clone(),
            room_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .context("desktop c leaked private game room")?;
        push_named_step(&mut steps, "outsider_isolation", started_at);

        let metrics_snapshot = if scenario.artifacts.metrics_snapshot {
            Some(
                runtime_b
                    .get_sync_status()
                    .await
                    .context("failed to collect final private-channel sync status")?,
            )
        } else {
            None
        };
        shutdown_runtime(runtime_a, "desktop a final shutdown")
            .await
            .context("desktop a final shutdown timed out")?;
        shutdown_runtime(runtime_b, "desktop b final shutdown")
            .await
            .context("desktop b final shutdown timed out")?;
        shutdown_runtime(runtime_c, "desktop c final shutdown")
            .await
            .context("desktop c final shutdown timed out")?;

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
    .context("scenario exceeded overall timeout")?
}

async fn shutdown_runtime(runtime: DesktopRuntime, label: &str) -> Result<()> {
    timeout(Duration::from_secs(30), runtime.shutdown())
        .await
        .with_context(|| format!("timed out waiting for {label}"))?;
    Ok(())
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

fn persist_runtime_identity(db_path: &Path, keys: &KukuriKeys) -> Result<()> {
    std::fs::write(
        db_path.with_extension("identity-key"),
        keys.export_secret_hex(),
    )
    .with_context(|| format!("failed to seed identity for {}", db_path.display()))
}

fn cleanup_runtime_artifacts(db_path: &Path) -> Result<()> {
    let config_paths = [
        db_path.to_path_buf(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
        db_path.with_extension("iroh-data"),
        db_path.with_extension("community-node.json"),
        db_path.with_extension("identity-store"),
        db_path.with_extension("identity-key"),
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
    if let (Some(parent), Some(stem)) = (db_path.parent(), db_path.file_stem()) {
        let stem = stem.to_string_lossy();
        let optional_secret_prefixes = [
            format!("{stem}.private-channel-capabilities-"),
            format!("{stem}.community-node-token-"),
        ];
        for entry in std::fs::read_dir(parent)
            .with_context(|| format!("failed to read {}", parent.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if optional_secret_prefixes
                .iter()
                .any(|prefix| file_name.starts_with(prefix))
            {
                if path.is_dir() {
                    std::fs::remove_dir_all(&path).with_context(|| {
                        format!("failed to remove stale directory {}", path.display())
                    })?;
                } else if path.exists() {
                    std::fs::remove_file(&path).with_context(|| {
                        format!("failed to remove stale file {}", path.display())
                    })?;
                }
            }
        }
    }
    Ok(())
}

fn remove_sqlite_runtime_db(db_path: &Path) -> Result<()> {
    for path in [
        db_path.to_path_buf(),
        db_path.with_extension("db-shm"),
        db_path.with_extension("db-wal"),
    ] {
        if !path.exists() {
            continue;
        }
        std::fs::remove_file(&path)
            .with_context(|| format!("failed to remove sqlite artifact {}", path.display()))?;
    }
    Ok(())
}

fn push_named_step(steps: &mut Vec<StepResult>, action: &str, started_at: Instant) {
    steps.push(StepResult {
        action: action.to_string(),
        duration_ms: started_at.elapsed().as_millis(),
    });
}

fn format_sync_snapshot(status: &SyncStatus, topic: &str) -> String {
    let topic_status = status
        .topic_diagnostics
        .iter()
        .find(|entry| entry.topic == topic)
        .map(|entry| {
            format!(
                "topic_peers={}, connected_peers={:?}, assist_peer_ids={:?}, configured_peer_ids={:?}, status_detail={}",
                entry.peer_count,
                entry.connected_peers,
                entry.assist_peer_ids,
                entry.configured_peer_ids,
                entry.status_detail
            )
        })
        .unwrap_or_else(|| "topic_status=missing".to_string());
    format!(
        "connected={}, peer_count={}, status_detail={}, last_error={:?}, discovery_connected_peers={:?}, {}",
        status.connected,
        status.peer_count,
        status.status_detail,
        status.last_error,
        status.discovery.connected_peer_ids,
        topic_status
    )
}

async fn wait_for_timeline_object(
    runtime: &DesktopRuntime,
    topic: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    let _ = wait_for_timeline_object_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        object_id,
        step_timeout,
    )
    .await?;
    Ok(())
}

async fn wait_for_timeline_object_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    object_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::PostView> {
    timeout(step_timeout, async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(50),
                })
                .await?;
            if let Some(item) = timeline
                .items
                .into_iter()
                .find(|item| item.object_id == object_id)
            {
                return Ok::<kukuri_app_api::PostView, anyhow::Error>(item);
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("timeline assertion timeout")?
}

async fn wait_for_topic_doc_index_entry(
    runtime: &DesktopRuntime,
    topic: &str,
    object_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            if runtime
                .has_topic_timeline_doc_index_entry(topic, object_id)
                .await
                .context("failed to query topic docs index")?
            {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .context("topic docs index assertion timeout")?
}

async fn assert_timeline_scope_excludes_object(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    object_id: &str,
    duration: Duration,
) -> Result<()> {
    let result = timeout(duration, async {
        loop {
            let timeline = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                    cursor: None,
                    limit: Some(50),
                })
                .await?;
            if timeline
                .items
                .iter()
                .any(|item| item.object_id == object_id)
            {
                anyhow::bail!("object leaked into filtered timeline scope");
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    match result {
        Err(_) => Ok(()),
        Ok(inner) => inner,
    }
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
    match timeout(step_timeout, async {
        let mut stable_ready_polls = 0usize;
        loop {
            let status = runtime.get_sync_status().await?;
            let ready = status.topic_diagnostics.iter().any(|entry| {
                let relay_assisted_ready = entry.assist_peer_ids.len() >= expected.min(1);
                entry.topic == topic
                    && entry.joined
                    && entry.peer_count >= expected
                    && (entry.connected_peers.len() >= expected.min(1) || relay_assisted_ready)
            });
            if ready {
                stable_ready_polls += 1;
                if stable_ready_polls >= 3 {
                    return Ok::<(), anyhow::Error>(());
                }
            } else {
                stable_ready_polls = 0;
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, topic))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!("topic connected-peer assertion timeout; {snapshot}");
        }
    }
}

#[cfg(test)]
fn is_retryable_friend_only_grant_import_error(message: &str) -> bool {
    message.contains("mutual relationship")
        || message.contains("friend-only grant epoch does not match the current policy")
        || message.contains("friend-only grant owner is not an active participant")
        || message.contains("timed out waiting for friend-only channel replica sync")
}

#[cfg(test)]
async fn wait_for_friend_only_grant_import(
    runtime: &DesktopRuntime,
    token: String,
    step_timeout: Duration,
) -> Result<kukuri_core::FriendOnlyGrantPreview> {
    let preview = kukuri_core::parse_friend_only_grant_token(token.as_str())?;
    match timeout(step_timeout, async {
        loop {
            match runtime
                .import_friend_only_grant(kukuri_desktop_runtime::ImportFriendOnlyGrantRequest {
                    token: token.clone(),
                })
                .await
            {
                Ok(preview) => return Ok::<_, anyhow::Error>(preview),
                Err(error)
                    if is_retryable_friend_only_grant_import_error(error.to_string().as_str()) =>
                {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => return Err(error),
            }
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let social_view = runtime
                .get_author_social_view(kukuri_desktop_runtime::AuthorRequest {
                    pubkey: preview.owner_pubkey.as_str().to_string(),
                })
                .await
                .ok()
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|| "social_view=unavailable".to_string());
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!(
                "friend-only grant import assertion timeout; owner_pubkey={}, {social_view}, {snapshot}",
                preview.owner_pubkey.as_str()
            );
        }
    }
}

#[cfg(test)]
async fn wait_for_friend_plus_share_import(
    runtime: &DesktopRuntime,
    token: String,
    step_timeout: Duration,
) -> Result<kukuri_core::FriendPlusSharePreview> {
    let preview = kukuri_core::parse_friend_plus_share_token(token.as_str())?;
    match timeout(step_timeout, async {
        loop {
            match runtime
                .import_friend_plus_share(kukuri_desktop_runtime::ImportFriendPlusShareRequest {
                    token: token.clone(),
                })
                .await
            {
                Ok(preview) => return Ok::<_, anyhow::Error>(preview),
                Err(error) if error.to_string().contains("mutual relationship") => {
                    sleep(Duration::from_millis(100)).await;
                }
                Err(error) => return Err(error),
            }
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            let social_view = runtime
                .get_author_social_view(kukuri_desktop_runtime::AuthorRequest {
                    pubkey: preview.sponsor_pubkey.as_str().to_string(),
                })
                .await
                .ok()
                .map(|value| {
                    format!(
                        "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                        value.following,
                        value.followed_by,
                        value.mutual,
                        value.friend_of_friend,
                        value.friend_of_friend_via_pubkeys
                    )
                })
                .unwrap_or_else(|| "social_view=unavailable".to_string());
            let snapshot = runtime
                .get_sync_status()
                .await
                .ok()
                .map(|status| format_sync_snapshot(&status, preview.topic_id.as_str()))
                .unwrap_or_else(|| "failed to read sync status".to_string());
            anyhow::bail!(
                "friend-plus share import assertion timeout; sponsor_pubkey={}, {social_view}, {snapshot}",
                preview.sponsor_pubkey.as_str()
            );
        }
    }
}

async fn wait_for_joined_private_channel(
    runtime: &DesktopRuntime,
    topic: &str,
    channel_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let joined = runtime
                .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                    topic: topic.to_string(),
                })
                .await?;
            if joined.iter().any(|entry| entry.channel_id == channel_id) {
                return Ok::<(), anyhow::Error>(());
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .context("joined private-channel assertion timeout")?
}

fn private_replication_retry_schedule(step_timeout: Duration) -> (usize, Duration) {
    let attempts = if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
        3
    } else {
        1
    };
    let per_attempt_timeout = if attempts > 1 {
        Duration::from_millis(
            (step_timeout.as_millis() / attempts as u128)
                .max(1)
                .try_into()
                .expect("private replication timeout fits in u64"),
        )
    } else {
        step_timeout
    };
    (attempts, per_attempt_timeout)
}

struct PublicReplicationLabels<'a> {
    failure: &'a str,
    publisher: &'a str,
    subscriber: &'a str,
}

async fn replicate_public_post_with_retry(
    publisher: &DesktopRuntime,
    subscriber: &DesktopRuntime,
    topic: &str,
    content_prefix: &str,
    step_timeout: Duration,
    labels: PublicReplicationLabels<'_>,
) -> Result<String> {
    let (attempts, attempt_timeout) = private_replication_retry_schedule(step_timeout);
    let mut last_error = None;

    for attempt in 1..=attempts {
        let attempt_result = async {
            let _ = publisher
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("failed to resubscribe publisher to public topic")?;
            let _ = subscriber
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: TimelineScope::Public,
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .context("failed to resubscribe subscriber to public topic")?;
            wait_for_topic_peer_count(publisher, topic, 1, attempt_timeout)
                .await
                .context("publisher did not observe public topic connectivity")?;
            wait_for_topic_peer_count(subscriber, topic, 1, attempt_timeout)
                .await
                .context("subscriber did not observe public topic connectivity")?;
            let post_id = publisher
                .create_post(CreatePostRequest {
                    topic: topic.to_string(),
                    content: format!("{content_prefix} #{attempt}"),
                    reply_to: None,
                    channel_ref: ChannelRef::Public,
                    attachments: Vec::new(),
                })
                .await
                .context("failed to create public post")?;
            wait_for_topic_doc_index_entry(publisher, topic, post_id.as_str(), attempt_timeout)
                .await
                .context("publisher did not persist public post into docs index")?;
            wait_for_timeline_object(subscriber, topic, post_id.as_str(), attempt_timeout)
                .await
                .context("timeline assertion timeout")?;
            Ok::<String, anyhow::Error>(post_id)
        }
        .await;

        match attempt_result {
            Ok(post_id) => return Ok(post_id),
            Err(error) if attempt < attempts => {
                last_error = Some(format!("{error:#}"));
                sleep(Duration::from_millis(250)).await;
            }
            Err(error) => {
                last_error = Some(format!("{error:#}"));
                break;
            }
        }
    }

    let publisher_status = publisher
        .get_sync_status()
        .await
        .ok()
        .map(|status| format_sync_snapshot(&status, topic))
        .unwrap_or_else(|| format!("failed to read {} sync status", labels.publisher));
    let subscriber_status = subscriber
        .get_sync_status()
        .await
        .ok()
        .map(|status| format_sync_snapshot(&status, topic))
        .unwrap_or_else(|| format!("failed to read {} sync status", labels.subscriber));
    Err(anyhow::anyhow!(
        "{}",
        last_error
            .unwrap_or_else(|| { format!("unknown replication failure for {}", labels.failure) })
    )
    .context(format!(
        "{} did not receive the {}; {}=({publisher_status}); {}=({subscriber_status})",
        labels.subscriber, labels.failure, labels.publisher, labels.subscriber
    )))
}

async fn refresh_private_channel_pair(
    runtime_a: &DesktopRuntime,
    runtime_b: &DesktopRuntime,
    ticket_a: &str,
    ticket_b: &str,
    topic: &str,
    private_scope: &TimelineScope,
) -> Result<()> {
    runtime_a
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_b.to_string(),
        })
        .await?;
    runtime_b
        .import_peer_ticket(ImportPeerTicketRequest {
            ticket: ticket_a.to_string(),
        })
        .await?;
    let _ = runtime_a
        .list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await;
    let _ = runtime_b
        .list_timeline(ListTimelineRequest {
            topic: topic.to_string(),
            scope: private_scope.clone(),
            cursor: None,
            limit: Some(20),
        })
        .await;
    Ok(())
}

async fn wait_for_live_session(
    runtime: &DesktopRuntime,
    topic: &str,
    session_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    let _ = wait_for_live_session_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        session_id,
        step_timeout,
    )
    .await?;
    Ok(())
}

async fn wait_for_live_session_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::LiveSessionView> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if let Some(session) = sessions
                .into_iter()
                .find(|session| session.session_id == session_id)
            {
                return Ok::<kukuri_app_api::LiveSessionView, anyhow::Error>(session);
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
    wait_for_live_viewer_count_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        session_id,
        expected,
        step_timeout,
    )
    .await
}

async fn wait_for_live_viewer_count_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    expected: usize,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
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
    wait_for_live_ended_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        session_id,
        step_timeout,
    )
    .await
}

async fn wait_for_live_ended_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    step_timeout: Duration,
) -> Result<()> {
    timeout(step_timeout, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
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

async fn assert_live_session_absent_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    session_id: &str,
    duration: Duration,
) -> Result<()> {
    let result = timeout(duration, async {
        loop {
            let sessions = runtime
                .list_live_sessions(ListLiveSessionsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if sessions
                .iter()
                .any(|session| session.session_id == session_id)
            {
                anyhow::bail!("live session leaked into filtered scope");
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    match result {
        Err(_) => Ok(()),
        Ok(inner) => inner,
    }
}

async fn wait_for_game_room(
    runtime: &DesktopRuntime,
    topic: &str,
    room_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::GameRoomView> {
    wait_for_game_room_in_scope(runtime, topic, TimelineScope::Public, room_id, step_timeout).await
}

async fn wait_for_game_room_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    room_id: &str,
    step_timeout: Duration,
) -> Result<kukuri_app_api::GameRoomView> {
    timeout(step_timeout, async {
        loop {
            let rooms = runtime
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
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
    wait_for_game_score_in_scope(
        runtime,
        topic,
        TimelineScope::Public,
        room_id,
        label,
        expected,
        step_timeout,
    )
    .await
}

async fn wait_for_game_score_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
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
                    scope: scope.clone(),
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

async fn assert_game_room_absent_in_scope(
    runtime: &DesktopRuntime,
    topic: &str,
    scope: TimelineScope,
    room_id: &str,
    duration: Duration,
) -> Result<()> {
    let result = timeout(duration, async {
        loop {
            let rooms = runtime
                .list_game_rooms(ListGameRoomsRequest {
                    topic: topic.to_string(),
                    scope: scope.clone(),
                })
                .await?;
            if rooms.iter().any(|room| room.room_id == room_id) {
                anyhow::bail!("game room leaked into filtered scope");
            }
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    match result {
        Err(_) => Ok(()),
        Ok(inner) => inner,
    }
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
    use kukuri_core::ChannelAudienceKind;
    use kukuri_desktop_runtime::{
        AuthorRequest, ExportFriendOnlyGrantRequest, ExportFriendPlusShareRequest,
        FreezePrivateChannelRequest, ImportFriendOnlyGrantRequest, ImportFriendPlusShareRequest,
        RotatePrivateChannelRequest,
    };
    use std::sync::Once;

    fn disable_keyring_for_tests() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| unsafe {
            std::env::set_var("KUKURI_DISABLE_KEYRING", "1");
        });
    }

    fn social_graph_propagation_timeout() -> Duration {
        if cfg!(target_os = "windows") || std::env::var_os("GITHUB_ACTIONS").is_some() {
            Duration::from_secs(300)
        } else {
            Duration::from_secs(30)
        }
    }

    async fn wait_for_connected_peer_count(runtime: &DesktopRuntime, expected: usize) {
        timeout(social_graph_propagation_timeout(), async {
            loop {
                let status = runtime.get_sync_status().await.expect("sync status");
                if status.connected && status.peer_count >= expected {
                    return;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("peer connection timeout");
    }

    async fn warm_author_social_view(runtime: &DesktopRuntime, author_pubkey: &str) {
        timeout(social_graph_propagation_timeout(), async {
            loop {
                if runtime
                    .get_author_social_view(AuthorRequest {
                        pubkey: author_pubkey.to_string(),
                    })
                    .await
                    .is_ok()
                {
                    return;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("author social view warmup timeout");
    }

    async fn wait_for_mutual_author_view(
        runtime: &DesktopRuntime,
        author_pubkey: &str,
        topic: &str,
    ) {
        match timeout(social_graph_propagation_timeout(), async {
            loop {
                let view = runtime
                    .get_author_social_view(AuthorRequest {
                        pubkey: author_pubkey.to_string(),
                    })
                    .await
                    .expect("author social view");
                if view.mutual {
                    return;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        {
            Ok(()) => {}
            Err(_) => {
                let social_view = runtime
                    .get_author_social_view(AuthorRequest {
                        pubkey: author_pubkey.to_string(),
                    })
                    .await
                    .ok()
                    .map(|value| {
                        format!(
                            "following={}, followed_by={}, mutual={}, friend_of_friend={}, fof_via={:?}",
                            value.following,
                            value.followed_by,
                            value.mutual,
                            value.friend_of_friend,
                            value.friend_of_friend_via_pubkeys
                        )
                    })
                    .unwrap_or_else(|| "social_view=unavailable".to_string());
                let snapshot = runtime
                    .get_sync_status()
                    .await
                    .ok()
                    .map(|status| format_sync_snapshot(&status, topic))
                    .unwrap_or_else(|| "failed to read sync status".to_string());
                panic!(
                    "mutual relationship timeout for {author_pubkey}; {social_view}, {snapshot}"
                );
            }
        }
    }

    #[tokio::test]
    async fn desktop_smoke_post_persist() {
        disable_keyring_for_tests();
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
        disable_keyring_for_tests();
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
        disable_keyring_for_tests();
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn private_channel_invite_connectivity() {
        disable_keyring_for_tests();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root");
        let artifacts = root
            .join("test-results")
            .join("kukuri")
            .join("private-channel-invite-connectivity");
        let result = run_named_scenario(root, "private_channel_invite_connectivity", &artifacts)
            .await
            .expect("scenario");

        assert_eq!(result.status, HarnessStatus::Pass);
        assert!(artifacts.join("result.json").exists());
    }

    #[tokio::test]
    async fn friend_only_rotate_requires_fresh_grant() {
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            // CI still covers fresh-grant rotation in app-api and desktop-runtime.
            return;
        }
        disable_keyring_for_tests();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("workspace root");
        let artifacts = root
            .join("test-results")
            .join("kukuri")
            .join("friend-only-rotate-connectivity");
        std::fs::create_dir_all(&artifacts).expect("create artifacts dir");

        let db_a = artifacts.join("friend-only-a.db");
        let db_b = artifacts.join("friend-only-b.db");
        let db_c = artifacts.join("friend-only-c.db");
        cleanup_runtime_artifacts(&db_a).expect("cleanup a");
        cleanup_runtime_artifacts(&db_b).expect("cleanup b");
        cleanup_runtime_artifacts(&db_c).expect("cleanup c");

        let runtime_a = DesktopRuntime::new_with_config(&db_a, TransportNetworkConfig::loopback())
            .await
            .expect("runtime a");
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .expect("runtime b");
        let runtime_c = DesktopRuntime::new_with_config(&db_c, TransportNetworkConfig::loopback())
            .await
            .expect("runtime c");

        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = runtime_b
            .local_peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        let ticket_c = runtime_c
            .local_peer_ticket()
            .await
            .expect("ticket c")
            .expect("ticket c value");

        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("a imports b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("b imports a");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_c.clone(),
            })
            .await
            .expect("a imports c");
        runtime_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("c imports a");

        let a_pubkey = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .local_author_pubkey;
        let b_pubkey = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .local_author_pubkey;
        let c_pubkey = runtime_c
            .get_sync_status()
            .await
            .expect("status c")
            .local_author_pubkey;

        wait_for_connected_peer_count(&runtime_a, 1).await;
        wait_for_connected_peer_count(&runtime_b, 1).await;
        wait_for_connected_peer_count(&runtime_c, 1).await;

        runtime_a
            .follow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("a follows b");
        runtime_b
            .follow_author(AuthorRequest {
                pubkey: a_pubkey.clone(),
            })
            .await
            .expect("b follows a");
        runtime_a
            .follow_author(AuthorRequest {
                pubkey: c_pubkey.clone(),
            })
            .await
            .expect("a follows c");
        runtime_c
            .follow_author(AuthorRequest {
                pubkey: a_pubkey.clone(),
            })
            .await
            .expect("c follows a");

        let topic = "kukuri:topic:harness-friend-only";
        let public_scope = TimelineScope::Public;
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe a");
        let _ = runtime_b
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe b");
        let _ = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("subscribe c");
        let topic_timeout = social_graph_propagation_timeout();
        wait_for_topic_peer_count(&runtime_a, topic, 1, topic_timeout)
            .await
            .expect("desktop a did not observe public topic connectivity");
        wait_for_topic_peer_count(&runtime_b, topic, 1, topic_timeout)
            .await
            .expect("desktop b did not observe public topic connectivity");
        wait_for_topic_peer_count(&runtime_c, topic, 1, topic_timeout)
            .await
            .expect("desktop c did not observe public topic connectivity");
        warm_author_social_view(&runtime_a, b_pubkey.as_str()).await;
        warm_author_social_view(&runtime_b, a_pubkey.as_str()).await;
        warm_author_social_view(&runtime_a, c_pubkey.as_str()).await;
        warm_author_social_view(&runtime_c, a_pubkey.as_str()).await;

        let channel = runtime_a
            .create_private_channel(CreatePrivateChannelRequest {
                topic: topic.to_string(),
                label: "friends".to_string(),
                audience_kind: ChannelAudienceKind::FriendOnly,
            })
            .await
            .expect("create friend-only channel");
        let old_grant = runtime_a
            .export_friend_only_grant(ExportFriendOnlyGrantRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export old grant");

        wait_for_friend_only_grant_import(
            &runtime_b,
            old_grant.clone(),
            social_graph_propagation_timeout(),
        )
        .await
        .expect("b imports old grant");
        wait_for_joined_private_channel(
            &runtime_b,
            topic,
            channel.channel_id.as_str(),
            topic_timeout,
        )
        .await
        .expect("desktop b did not join friend-only channel");

        let private_scope = TimelineScope::Channel {
            channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
        };
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
        };
        let private_post_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "friend-only history".to_string(),
                reply_to: None,
                channel_ref: private_ref,
                attachments: Vec::new(),
            })
            .await
            .expect("create friend-only post");
        wait_for_timeline_object_in_scope(
            &runtime_a,
            topic,
            private_scope.clone(),
            private_post_id.as_str(),
            Duration::from_secs(10),
        )
        .await
        .expect("desktop a did not receive friend-only post");
        assert_timeline_scope_excludes_object(
            &runtime_c,
            topic,
            public_scope.clone(),
            private_post_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .expect("desktop c public scope leaked friend-only post");

        runtime_a
            .unfollow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("a unfollows b");
        let joined_a = timeout(Duration::from_secs(10), async {
            loop {
                let joined = runtime_a
                    .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                        topic: topic.to_string(),
                    })
                    .await
                    .expect("list joined channels on a");
                if joined.iter().any(|entry| {
                    entry.channel_id == channel.channel_id
                        && entry.rotation_required
                        && entry.stale_participant_count == 1
                }) {
                    return joined;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("rotation required timeout");
        assert!(joined_a.iter().any(|entry| {
            entry.channel_id == channel.channel_id
                && entry.rotation_required
                && entry.stale_participant_count == 1
        }));

        let rotated = runtime_a
            .rotate_private_channel(RotatePrivateChannelRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
            })
            .await
            .expect("rotate friend-only channel");

        runtime_c
            .import_friend_only_grant(ImportFriendOnlyGrantRequest { token: old_grant })
            .await
            .expect_err("old grant should fail after rotate");

        let fresh_grant = runtime_a
            .export_friend_only_grant(ExportFriendOnlyGrantRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export fresh grant");
        let _ = runtime_a
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("resubscribe a before fresh grant");
        let _ = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: public_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("resubscribe c before fresh grant");
        wait_for_topic_peer_count(&runtime_a, topic, 1, topic_timeout)
            .await
            .expect("desktop a did not observe public topic connectivity after rotate");
        wait_for_topic_peer_count(&runtime_c, topic, 1, topic_timeout)
            .await
            .expect("desktop c did not observe public topic connectivity after rotate");
        warm_author_social_view(&runtime_a, c_pubkey.as_str()).await;
        warm_author_social_view(&runtime_c, a_pubkey.as_str()).await;
        wait_for_mutual_author_view(&runtime_a, c_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&runtime_c, a_pubkey.as_str(), topic).await;
        let fresh_preview = wait_for_friend_only_grant_import(
            &runtime_c,
            fresh_grant,
            social_graph_propagation_timeout(),
        )
        .await
        .expect("c imports fresh grant");
        assert_eq!(fresh_preview.epoch_id, rotated.current_epoch_id);

        let joined_c = runtime_c
            .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                topic: topic.to_string(),
            })
            .await
            .expect("list joined channels on c");
        let channel_c = joined_c
            .iter()
            .find(|entry| entry.channel_id == channel.channel_id)
            .expect("friend-only channel on c");
        assert_eq!(channel_c.current_epoch_id, rotated.current_epoch_id);
        assert!(channel_c.archived_epoch_ids.is_empty());

        let c_private_timeline = runtime_c
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: private_scope.clone(),
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("list c private timeline");
        assert!(
            c_private_timeline
                .items
                .iter()
                .all(|item| item.object_id != private_post_id)
        );

        shutdown_runtime(runtime_a, "friend-only harness runtime a")
            .await
            .expect("shutdown runtime a");
        shutdown_runtime(runtime_b, "friend-only harness runtime b")
            .await
            .expect("shutdown runtime b");
        shutdown_runtime(runtime_c, "friend-only harness runtime c")
            .await
            .expect("shutdown runtime c");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn friend_plus_share_freeze_rotate_connectivity() {
        if std::env::var_os("GITHUB_ACTIONS").is_some() {
            // CI still covers freeze/rotate/share recovery in app-api and desktop-runtime.
            return;
        }
        disable_keyring_for_tests();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_a = dir.path().join("friend-plus-harness-a.db");
        let db_b = dir.path().join("friend-plus-harness-b.db");
        let db_c = dir.path().join("friend-plus-harness-c.db");
        let db_d = dir.path().join("friend-plus-harness-d.db");
        let runtime_a = DesktopRuntime::new_with_config(&db_a, TransportNetworkConfig::loopback())
            .await
            .expect("runtime a");
        let runtime_b = DesktopRuntime::new_with_config(&db_b, TransportNetworkConfig::loopback())
            .await
            .expect("runtime b");
        let runtime_c = DesktopRuntime::new_with_config(&db_c, TransportNetworkConfig::loopback())
            .await
            .expect("runtime c");
        let runtime_d = DesktopRuntime::new_with_config(&db_d, TransportNetworkConfig::loopback())
            .await
            .expect("runtime d");

        let ticket_a = runtime_a
            .local_peer_ticket()
            .await
            .expect("ticket a")
            .expect("ticket a value");
        let ticket_b = runtime_b
            .local_peer_ticket()
            .await
            .expect("ticket b")
            .expect("ticket b value");
        let ticket_c = runtime_c
            .local_peer_ticket()
            .await
            .expect("ticket c")
            .expect("ticket c value");
        let ticket_d = runtime_d
            .local_peer_ticket()
            .await
            .expect("ticket d")
            .expect("ticket d value");

        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("a imports b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("b imports a");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_c.clone(),
            })
            .await
            .expect("a imports c");
        runtime_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("c imports a");
        runtime_a
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_d.clone(),
            })
            .await
            .expect("a imports d");
        runtime_d
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_a.clone(),
            })
            .await
            .expect("d imports a");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_c.clone(),
            })
            .await
            .expect("b imports c");
        runtime_c
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("c imports b");
        runtime_b
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_d.clone(),
            })
            .await
            .expect("b imports d");
        runtime_d
            .import_peer_ticket(ImportPeerTicketRequest {
                ticket: ticket_b.clone(),
            })
            .await
            .expect("d imports b");

        let a_pubkey = runtime_a
            .get_sync_status()
            .await
            .expect("status a")
            .local_author_pubkey;
        let b_pubkey = runtime_b
            .get_sync_status()
            .await
            .expect("status b")
            .local_author_pubkey;
        let c_pubkey = runtime_c
            .get_sync_status()
            .await
            .expect("status c")
            .local_author_pubkey;
        let d_pubkey = runtime_d
            .get_sync_status()
            .await
            .expect("status d")
            .local_author_pubkey;
        let topic = "kukuri:topic:harness-friend-plus";

        wait_for_connected_peer_count(&runtime_a, 1).await;
        wait_for_connected_peer_count(&runtime_b, 1).await;
        wait_for_connected_peer_count(&runtime_c, 1).await;
        wait_for_connected_peer_count(&runtime_d, 1).await;

        runtime_a
            .follow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("a follows b");
        runtime_b
            .follow_author(AuthorRequest {
                pubkey: a_pubkey.clone(),
            })
            .await
            .expect("b follows a");
        runtime_b
            .follow_author(AuthorRequest {
                pubkey: c_pubkey.clone(),
            })
            .await
            .expect("b follows c");
        runtime_c
            .follow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("c follows b");
        runtime_b
            .follow_author(AuthorRequest {
                pubkey: d_pubkey.clone(),
            })
            .await
            .expect("b follows d");
        runtime_d
            .follow_author(AuthorRequest {
                pubkey: b_pubkey.clone(),
            })
            .await
            .expect("d follows b");

        wait_for_mutual_author_view(&runtime_b, a_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&runtime_c, b_pubkey.as_str(), topic).await;
        wait_for_mutual_author_view(&runtime_d, b_pubkey.as_str(), topic).await;

        let public_scope = TimelineScope::Public;
        for runtime in [&runtime_a, &runtime_b, &runtime_c, &runtime_d] {
            let _ = runtime
                .list_timeline(ListTimelineRequest {
                    topic: topic.to_string(),
                    scope: public_scope.clone(),
                    cursor: None,
                    limit: Some(20),
                })
                .await
                .expect("subscribe runtime");
        }
        let topic_timeout = social_graph_propagation_timeout();
        wait_for_topic_peer_count(&runtime_a, topic, 1, topic_timeout)
            .await
            .expect("desktop a did not observe public topic connectivity");
        wait_for_topic_peer_count(&runtime_b, topic, 1, topic_timeout)
            .await
            .expect("desktop b did not observe public topic connectivity");
        wait_for_topic_peer_count(&runtime_c, topic, 1, topic_timeout)
            .await
            .expect("desktop c did not observe public topic connectivity");
        wait_for_topic_peer_count(&runtime_d, topic, 1, topic_timeout)
            .await
            .expect("desktop d did not observe public topic connectivity");

        let channel = runtime_a
            .create_private_channel(CreatePrivateChannelRequest {
                topic: topic.to_string(),
                label: "friends+".to_string(),
                audience_kind: ChannelAudienceKind::FriendPlus,
            })
            .await
            .expect("create friend-plus channel");
        let share_ab = runtime_a
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export a->b share");
        wait_for_friend_plus_share_import(&runtime_b, share_ab, social_graph_propagation_timeout())
            .await
            .expect("b imports share");
        let share_bc = runtime_b
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export b->c share");
        wait_for_friend_plus_share_import(&runtime_c, share_bc, social_graph_propagation_timeout())
            .await
            .expect("c imports share");
        let stale_share_for_d = runtime_b
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export b->d share");

        let private_scope = TimelineScope::Channel {
            channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
        };
        let private_ref = ChannelRef::PrivateChannel {
            channel_id: kukuri_core::ChannelId::new(channel.channel_id.clone()),
        };
        let old_post_id = runtime_a
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "friend-plus history".to_string(),
                reply_to: None,
                channel_ref: private_ref.clone(),
                attachments: Vec::new(),
            })
            .await
            .expect("create friend-plus post");
        wait_for_timeline_object_in_scope(
            &runtime_b,
            topic,
            private_scope.clone(),
            old_post_id.as_str(),
            Duration::from_secs(10),
        )
        .await
        .expect("b receives old post");
        wait_for_timeline_object_in_scope(
            &runtime_c,
            topic,
            private_scope.clone(),
            old_post_id.as_str(),
            Duration::from_secs(10),
        )
        .await
        .expect("c receives old post");
        assert_timeline_scope_excludes_object(
            &runtime_d,
            topic,
            public_scope.clone(),
            old_post_id.as_str(),
            Duration::from_millis(500),
        )
        .await
        .expect("public scope leaked friend-plus post");

        let frozen = runtime_a
            .freeze_private_channel(FreezePrivateChannelRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
            })
            .await
            .expect("freeze friend-plus channel");
        assert_eq!(
            frozen.sharing_state,
            kukuri_core::ChannelSharingState::Frozen
        );

        let freeze_post_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "friend-plus frozen write".to_string(),
                reply_to: None,
                channel_ref: private_ref.clone(),
                attachments: Vec::new(),
            })
            .await
            .expect("write should continue after freeze");
        wait_for_timeline_object_in_scope(
            &runtime_c,
            topic,
            private_scope.clone(),
            freeze_post_id.as_str(),
            Duration::from_secs(10),
        )
        .await
        .expect("c receives frozen write");

        runtime_d
            .import_friend_plus_share(ImportFriendPlusShareRequest {
                token: stale_share_for_d.clone(),
            })
            .await
            .expect_err("frozen share should fail");

        let rotated = runtime_a
            .rotate_private_channel(RotatePrivateChannelRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
            })
            .await
            .expect("rotate friend-plus channel");

        timeout(Duration::from_secs(10), async {
            loop {
                let joined = runtime_c
                    .list_joined_private_channels(ListJoinedPrivateChannelsRequest {
                        topic: topic.to_string(),
                    })
                    .await
                    .expect("list joined channels on c");
                if joined.iter().any(|entry| {
                    entry.channel_id == channel.channel_id
                        && entry.current_epoch_id == rotated.current_epoch_id
                        && entry.joined_via_pubkey.as_deref() == Some(b_pubkey.as_str())
                }) {
                    return;
                }
                sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("c rotation redeem timeout");

        runtime_d
            .import_friend_plus_share(ImportFriendPlusShareRequest {
                token: stale_share_for_d,
            })
            .await
            .expect_err("old share should fail after rotate");

        let new_post_id = runtime_b
            .create_post(CreatePostRequest {
                topic: topic.to_string(),
                content: "friend-plus new".to_string(),
                reply_to: None,
                channel_ref: private_ref,
                attachments: Vec::new(),
            })
            .await
            .expect("create new epoch post");
        wait_for_timeline_object_in_scope(
            &runtime_c,
            topic,
            private_scope.clone(),
            new_post_id.as_str(),
            Duration::from_secs(10),
        )
        .await
        .expect("c receives new epoch post");

        let fresh_share = runtime_b
            .export_friend_plus_share(ExportFriendPlusShareRequest {
                topic: topic.to_string(),
                channel_id: channel.channel_id.clone(),
                expires_at: None,
            })
            .await
            .expect("export fresh share");
        wait_for_friend_plus_share_import(
            &runtime_d,
            fresh_share,
            social_graph_propagation_timeout(),
        )
        .await
        .expect("d imports fresh share");
        let d_private_timeline = runtime_d
            .list_timeline(ListTimelineRequest {
                topic: topic.to_string(),
                scope: private_scope,
                cursor: None,
                limit: Some(20),
            })
            .await
            .expect("list d private timeline");
        assert!(
            d_private_timeline
                .items
                .iter()
                .all(|item| item.object_id != old_post_id)
        );
        assert!(
            d_private_timeline
                .items
                .iter()
                .any(|item| item.object_id == new_post_id)
        );

        shutdown_runtime(runtime_a, "friend-plus harness runtime a")
            .await
            .expect("shutdown runtime a");
        shutdown_runtime(runtime_b, "friend-plus harness runtime b")
            .await
            .expect("shutdown runtime b");
        shutdown_runtime(runtime_c, "friend-plus harness runtime c")
            .await
            .expect("shutdown runtime c");
        shutdown_runtime(runtime_d, "friend-plus harness runtime d")
            .await
            .expect("shutdown runtime d");
    }
}
