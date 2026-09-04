mod accounts;
mod attachments;
mod backup;
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

pub use accounts::{
    AccountKeyExport, AccountKeyImportPreview, AccountRecord, AccountsSnapshot, account_db_path,
    add_account_from_env, ensure_accounts_initialized_from_env, import_account_key_from_env,
    list_accounts, preview_account_key_import, set_active_account,
};
pub use backup::{
    CreateDeviceBackupRequest, DeviceBackupCancellation, DeviceBackupPhase, DeviceBackupPreview,
    DeviceBackupProgress, DeviceBackupRestoreResult, DeviceBackupSummary, DeviceRestorePhase,
    InstalledDeviceRestore, PreparedDeviceRestore, PreviewDeviceBackupRequest,
    RestoreDeviceBackupRequest, acknowledge_pending_device_restore_frontend_state,
    commit_device_restore, create_device_backup, finalize_device_restore,
    finalize_pending_device_restore, install_prepared_device_restore,
    mark_device_restore_activated, mark_device_restore_awaiting_consent,
    pending_device_restore_frontend_state, pending_device_restore_phase, prepare_device_restore,
    preview_device_backup, recover_interrupted_restore, rollback_device_restore,
    rollback_pending_device_restore, validate_prepared_device_restore,
};
pub use community_node::{
    AcceptCommunityNodeConsentsRequest, CommunityNodeAdmissionRejection,
    CommunityNodeAdmissionRejectionCode, CommunityNodeAuthState, CommunityNodeAuthorityScope,
    CommunityNodeCapabilityScope, CommunityNodeConfig, CommunityNodeConsentDocumentRef,
    CommunityNodeIndexQueryError, CommunityNodeIndexQueryRequest, CommunityNodeIndexingRequest,
    CommunityNodeIndexingRequestError, CommunityNodeLegalDocument, CommunityNodeLocalConsentRecord,
    CommunityNodeLocalConsentState, CommunityNodeManifest, CommunityNodeManifestFetch,
    CommunityNodeManifestFetchStatus, CommunityNodeNodeConfig, CommunityNodeNodeStatus,
    CommunityNodeP2pBoundary, CommunityNodePoliciesResponse, CommunityNodePolicyDocument,
    CommunityNodeRelationNeighborsRequest, CommunityNodeReportAppeal, CommunityNodeReportError,
    CommunityNodeSessionPhase, CommunityNodeTargetRequest, CommunityNodeTesterFeedbackError,
    CommunityNodeTesterFeedbackResponse, CommunityNodeTesterFeedbackSubmission,
    CommunityNodeTrustRelationError, CommunityNodeUserAdvisoryRequest, DomeHostingRequestError,
    FetchCommunityNodePoliciesRequest, IndexEntryView, IndexQueryResponse, IndexScopeKind,
    RelationNeighborsResponse, RelationOptoutResponse, RelationReadResponse,
    SetCommunityNodeConfigNode, SetCommunityNodeConfigRequest, SetCommunityNodeInviteCodeRequest,
    SubmitCommunityNodeReportRequest, SubmitCommunityNodeReportResult,
    SubmitCommunityNodeReportStatus, SubmitIndexingRequestResponse, TrustUserReadResponse,
};
pub use discovery::{DiscoveryConfig, SetDiscoverySeedsRequest};
// 起動エラーの typed 分類(WP-Q2)。src-tauri は downcast で DatabaseOpen/Migration を判定する。
pub use kukuri_store::StoreStartupError;
pub use paths::{resolve_app_data_dir_from_env, resolve_db_path_from_env};
pub use requests::{
    AbortDomeTransitionRequest, AcceptDomeConnectionProposalRequest, AuthorRequest,
    BookmarkCustomReactionRequest, BookmarkPostRequest, CloseDomeHostingRequest,
    CommitDomeLayoutRequest, CommitDomeTransitionRequest, CreateAttachmentRequest,
    CreateCustomReactionAssetRequest, CreateDomeConnectionProposalRequest, CreateGameRoomRequest,
    CreateLiveSessionRequest, CreateMetaverseRoomRequest, CreatePostRequest,
    CreatePrivateChannelRequest, CreateRepostRequest, CustomReactionCropRect,
    DelegateDomeHostingRequest, DeleteDirectMessageMessageRequest, DirectMessageRequest,
    ExportAccountKeyRequest, ExportChannelAccessTokenRequest, ExportFriendOnlyGrantRequest,
    ExportFriendPlusShareRequest, ExportPrivateChannelInviteRequest, FreezePrivateChannelRequest,
    GetBlobMediaRequest, GetBlobPreviewRequest, GetDomeHostingRequest, ImportAccountKeyRequest,
    ImportChannelAccessTokenRequest, ImportFriendOnlyGrantRequest, ImportFriendPlusShareRequest,
    ImportMetaverseRoomAssetRequest, ImportPeerTicketRequest, ImportPrivateChannelInviteRequest,
    LeavePrivateChannelRequest, ListDirectMessageMessagesRequest,
    ListDomeConnectionTopologyRequest, ListGameRoomsRequest, ListJoinedPrivateChannelsRequest,
    ListLiveSessionsRequest, ListMetaverseRoomEventsRequest, ListProfileTimelineRequest,
    ListRecentReactionsRequest, ListSocialConnectionsRequest, ListThreadRequest,
    ListTimelineRequest, LiveSessionCommandRequest, MoveDomeRequest, NotificationIdRequest,
    PostWithdrawalReasonRequest, PrepareDomeTransitionRequest, PreviewAccountKeyImportRequest,
    PreviewChannelAccessTokenRequest, PublishMetaverseRoomEventRequest, ReactionKeyRequest,
    RemoveBookmarkedCustomReactionRequest, RemoveBookmarkedPostRequest,
    ResolveCommunityIndexPostsRequest, ResyncDomeSnapshotsRequest, RevokeDomeConnectionRequest,
    RotatePrivateChannelRequest, SendDirectMessageRequest, SetChannelGossipEnabledRequest,
    SetMyProfileRequest, SetPrivateChannelEntryDomeRequest, SetTopicGossipEnabledRequest,
    StartOwnerDomeHostingRequest, SubmitDomeSessionInputRequest, SwitchAccountRequest,
    ToggleReactionRequest, UnsubscribeTopicRequest, UpdateGameRoomRequest,
    UpdateMetaverseRoomRequest, WithdrawDomeConnectionProposalRequest, WithdrawPostRequest,
    WithdrawalReasonVisibilityRequest,
};
pub use runtime::{DesktopRuntime, RuntimeEvent};
