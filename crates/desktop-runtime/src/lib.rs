mod attachments;
mod community_node;
mod discovery;
mod identity;
#[cfg(feature = "ts")]
mod ipc_ts_export;
mod paths;
mod requests;
mod runtime;
mod stack;

#[cfg(test)]
mod tests;

pub use community_node::{
    AcceptCommunityNodeConsentsRequest, CommunityNodeAdmissionRejection,
    CommunityNodeAdmissionRejectionCode, CommunityNodeAuthState, CommunityNodeAuthorityScope,
    CommunityNodeCapabilityScope, CommunityNodeConfig, CommunityNodeIndexQueryError,
    CommunityNodeIndexQueryRequest, CommunityNodeIndexingRequest,
    CommunityNodeIndexingRequestError, CommunityNodeManifest, CommunityNodeManifestFetch,
    CommunityNodeManifestFetchStatus, CommunityNodeNodeConfig, CommunityNodeNodeStatus,
    CommunityNodeP2pBoundary, CommunityNodeRelationNeighborsRequest, CommunityNodeReportAppeal,
    CommunityNodeReportError, CommunityNodeSessionPhase, CommunityNodeTargetRequest,
    CommunityNodeTesterFeedbackError, CommunityNodeTesterFeedbackResponse,
    CommunityNodeTesterFeedbackSubmission, CommunityNodeTrustRelationError,
    CommunityNodeUserAdvisoryRequest, DomeHostingRequestError, IndexEntryView, IndexQueryResponse,
    IndexScopeKind, RelationNeighborsResponse, RelationOptoutResponse, RelationReadResponse,
    SetCommunityNodeConfigNode, SetCommunityNodeConfigRequest, SetCommunityNodeInviteCodeRequest,
    SubmitCommunityNodeReportRequest, SubmitCommunityNodeReportResult,
    SubmitCommunityNodeReportStatus, SubmitIndexingRequestResponse, TrustUserReadResponse,
};
pub use discovery::{DiscoveryConfig, SetDiscoverySeedsRequest};
// 起動エラーの typed 分類(WP-Q2)。src-tauri は downcast で DatabaseOpen/Migration を判定する。
pub use kukuri_store::StoreStartupError;
pub use paths::resolve_db_path_from_env;
pub use requests::{
    AbortDomeTransitionRequest, AcceptDomeConnectionProposalRequest, AuthorRequest,
    BookmarkCustomReactionRequest, BookmarkPostRequest, CloseDomeHostingRequest,
    CommitDomeLayoutRequest, CommitDomeTransitionRequest, CreateAttachmentRequest,
    CreateCustomReactionAssetRequest, CreateDomeConnectionProposalRequest, CreateGameRoomRequest,
    CreateLiveSessionRequest, CreateMetaverseRoomRequest, CreatePostRequest,
    CreatePrivateChannelRequest, CreateRepostRequest, CustomReactionCropRect,
    DelegateDomeHostingRequest, DeleteDirectMessageMessageRequest, DirectMessageRequest,
    ExportChannelAccessTokenRequest, ExportFriendOnlyGrantRequest, ExportFriendPlusShareRequest,
    ExportPrivateChannelInviteRequest, FreezePrivateChannelRequest, GetBlobMediaRequest,
    GetBlobPreviewRequest, GetDomeHostingRequest, ImportChannelAccessTokenRequest,
    ImportFriendOnlyGrantRequest, ImportFriendPlusShareRequest, ImportMetaverseRoomAssetRequest,
    ImportPeerTicketRequest, ImportPrivateChannelInviteRequest, LeavePrivateChannelRequest,
    ListDirectMessageMessagesRequest, ListDomeConnectionTopologyRequest, ListGameRoomsRequest,
    ListJoinedPrivateChannelsRequest, ListLiveSessionsRequest, ListMetaverseRoomEventsRequest,
    ListProfileTimelineRequest, ListRecentReactionsRequest, ListSocialConnectionsRequest,
    ListThreadRequest, ListTimelineRequest, LiveSessionCommandRequest, MoveDomeRequest,
    NotificationIdRequest, PostWithdrawalReasonRequest, PrepareDomeTransitionRequest,
    PreviewChannelAccessTokenRequest, PublishMetaverseRoomEventRequest, ReactionKeyRequest,
    RemoveBookmarkedCustomReactionRequest, RemoveBookmarkedPostRequest,
    ResolveCommunityIndexPostsRequest, ResyncDomeSnapshotsRequest, RevokeDomeConnectionRequest,
    RotatePrivateChannelRequest, SendDirectMessageRequest, SetChannelGossipEnabledRequest,
    SetMyProfileRequest, SetPrivateChannelEntryDomeRequest, SetTopicGossipEnabledRequest,
    StartOwnerDomeHostingRequest, SubmitDomeSessionInputRequest, ToggleReactionRequest,
    UnsubscribeTopicRequest, UpdateGameRoomRequest, UpdateMetaverseRoomRequest,
    WithdrawDomeConnectionProposalRequest, WithdrawPostRequest, WithdrawalReasonVisibilityRequest,
};
pub use runtime::{DesktopRuntime, RuntimeEvent};
