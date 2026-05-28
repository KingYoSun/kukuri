use crate::*;

pub(crate) fn step_name(step: &ScenarioStep) -> &'static str {
    match step {
        ScenarioStep::LaunchDesktop => "launch_desktop",
        ScenarioStep::SelectTopic { .. } => "select_topic",
        ScenarioStep::SelectPublicTimeline => "select_public_timeline",
        ScenarioStep::CreatePrivateChannel { .. } => "create_private_channel",
        ScenarioStep::SelectPrivateChannel { .. } => "select_private_channel",
        ScenarioStep::CreatePost { .. } => "create_post",
        ScenarioStep::AssertTimelineContains { .. } => "assert_timeline_contains",
        ScenarioStep::BookmarkPost { .. } => "bookmark_post",
        ScenarioStep::AssertBookmarkListContains { .. } => "assert_bookmark_list_contains",
        ScenarioStep::AssertBookmarkListMissing { .. } => "assert_bookmark_list_missing",
        ScenarioStep::RemoveBookmark { .. } => "remove_bookmark",
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

pub(crate) fn parse_game_status(value: &str) -> Result<GameRoomStatus> {
    match value {
        "Open" | "Waiting" => Ok(GameRoomStatus::Waiting),
        "InProgress" | "Running" => Ok(GameRoomStatus::Running),
        "Paused" => Ok(GameRoomStatus::Paused),
        "Finished" | "Ended" => Ok(GameRoomStatus::Ended),
        _ => anyhow::bail!("unsupported game room status: {value}"),
    }
}
