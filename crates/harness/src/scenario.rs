use crate::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioKind {
    #[default]
    DesktopSmoke,
    CommunityNodePublicConnectivity,
    CommunityNodeMultiDeviceConnectivity,
    PrivateChannelInviteConnectivity,
    PairwiseDirectMessageConnectivity,
    CommunityNodeIndexQueryClient,
    CommunityNodeReportRouting,
    CommunityNodeTrustRelationClient,
    DomeHostingLifecycle,
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
    SelectPublicTimeline,
    CreatePrivateChannel {
        label: String,
    },
    SelectPrivateChannel {
        label: String,
    },
    CreatePost {
        content: String,
    },
    AssertTimelineContains {
        text: String,
    },
    BookmarkPost {
        content: String,
    },
    AssertBookmarkListContains {
        text: String,
    },
    AssertBookmarkListMissing {
        text: String,
    },
    RemoveBookmark {
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
    CreateMetaverseDome {
        title: String,
        description: String,
        max_peers: Option<u32>,
    },
    CustomizeMetaverseDome {
        title: String,
        gravity_milli: u32,
        wall_material: String,
        prop_position: [i64; 3],
    },
    AssertMetaverseDome {
        title: String,
        gravity_milli: u32,
        wall_material: String,
        prop_position: [i64; 3],
    },
    AssertMetaverseDomeRejectsInvalid {
        title: String,
    },
    AssertMetaverseDomeCreateRejected {
        title: String,
    },
    MoveMetaverseDome {
        title: String,
        move_id: String,
        target_topic: String,
        target_channel_label: Option<String>,
    },
    ExerciseDomeConnections {
        local_title: String,
    },
    ExerciseDomeTransition {
        local_title: String,
    },
    AssertDomeConnectionTopology {
        component_count: usize,
        active_connection_count: usize,
    },
    RevokeLocalDomeConnection,
    AssertMetaverseDomeMissing {
        title: String,
    },
    RestartDesktop,
    SearchCommunityIndex {
        query: String,
        scope_kind: String,
        scope_id: String,
        expect_object_id: String,
    },
    DiscoverCommunityIndex {
        expect_object_id: String,
    },
    RecommendCommunityIndex {
        expect_object_id: String,
    },
    AssertCommunityIndexError {
        query: String,
        code: String,
    },
    AssertNoReportProvenance,
    AssertObservedReportRouting {
        expect_capability: String,
        #[serde(default)]
        expect_subject_kinds: Vec<String>,
    },
    ReadCommunityTrust {
        target_pubkey: String,
        expect_trust_millis: i64,
    },
    ReadCommunityRelation {
        target_pubkey: String,
        expect_score_millis: i64,
    },
    AssertCommunityRelationNeighbor {
        pubkey: String,
    },
    AssertRelationOptout {
        operation: String,
        expect_enabled: bool,
    },
    AssertTrustRelationError {
        endpoint: String,
        target_pubkey: String,
        code: String,
    },
    /// リスク判定への匿名の異議申し立てを送り、受理された判定識別子を確認する(#704)。
    SubmitCommunityAppeal {
        target_pubkey: String,
        risk_signal_id: String,
    },
    /// 信頼評価を再取得し、根拠の異議申し立て状態と寄与を確認する(#704)。
    AssertTrustBasisAppeal {
        target_pubkey: String,
        signal_id: String,
        expect_status: String,
        expect_contribution_zero: bool,
    },
    /// 運営者の審査(認容)をスタブ上で確定させる(#704。実効果はサーバ側結合試験で固定済み)。
    ResolveCommunityAppeal,
    /// 索引検索の件数だけを確認する(距離利用停止の結線確認。#704)。
    AssertCommunityIndexEntryCount {
        query: String,
        scope_kind: String,
        scope_id: String,
        expect_entry_count: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioScoreUpdate {
    pub label: String,
    pub score: i64,
}

pub fn load_scenario(path: &Path) -> Result<ScenarioSpec> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario {}", path.display()))?;
    serde_yaml::from_str(&contents).context("failed to parse scenario yaml")
}
