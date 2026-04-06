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
    RestartDesktop,
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
