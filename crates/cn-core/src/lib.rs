mod admission;
mod auth;
mod bootstrap;
mod config;
mod consents;
mod database;
mod env;
mod errors;
mod index_scope;
mod models;
mod normalize;
mod rendezvous;
mod reports;
mod rollout;
mod safety_events;
#[cfg(test)]
mod tests;

pub use admission::{
    AdmissionConfig, AdmissionMode, AdmissionRejection, AllowlistEntry, BannedEntry,
    InviteCodeSummary, add_allowlist, ban_subscriber, invite_code_hash, issue_invite_code,
    list_allowlist, list_banned, list_invite_codes, load_admission_config, remove_allowlist,
    revoke_invite_code, set_admission_mode, unban_subscriber,
};
pub use auth::{
    build_auth_envelope_json, create_auth_challenge, require_bearer_identity,
    require_bearer_pubkey, verify_auth_envelope_and_issue_token,
};
pub use bootstrap::{
    load_bootstrap_nodes, load_bootstrap_seed_peers, refresh_bootstrap_peer_registration,
    upsert_bootstrap_node,
};
pub use config::{
    AUTH_CHALLENGE_TTL_SECONDS, AUTH_ENVELOPE_KIND, AUTH_EVENT_MAX_SKEW_SECONDS, AuthMode,
    AuthRolloutConfig, BOOTSTRAP_PEER_REGISTRATION_TTL_SECONDS,
    COMMUNITY_NODE_ADMISSION_SERVICE_NAME, COMMUNITY_NODE_AUTH_SERVICE_NAME,
    COMMUNITY_NODE_DATABASE_INIT_MODE_ENV, COMMUNITY_NODE_RENDEZVOUS_KEY_PREFIX_ENV,
    COMMUNITY_NODE_RENDEZVOUS_REDIS_URL_ENV, DEFAULT_TOKEN_TTL_SECONDS, DatabaseInitMode,
    JwtConfig, TOPIC_RENDEZVOUS_TTL_SECONDS, USER_API_BEARER_CHALLENGE,
};
pub use consents::{accept_consents, get_consent_status, require_consents};
pub use database::{
    TestDatabase, connect_postgres, ensure_database_ready, initialize_database,
    initialize_database_for_runtime, migrate_postgres, seed_default_policies,
};
pub use env::{parse_bool_env, parse_csv_env};
pub use errors::{ApiError, ApiResult, auth_required_error, consent_required_error};
pub use index_scope::{
    ChannelSecret, ChannelSecretCipher, ChannelSecretConflict, IndexScopeKind, IndexingRequest,
    IndexingRequestStatus, SupportedTopic, add_supported_topic, approve_indexing_request,
    get_channel_secret, insert_indexing_request, is_topic_supported, list_channel_secrets,
    list_indexing_requests, list_supported_topics, register_channel_secret,
    reject_indexing_request, remove_channel_secret, remove_supported_topic, upsert_channel_secret,
};
pub use models::{
    AuthChallengeResponse, AuthVerifyResponse, BearerIdentity, BootstrapHeartbeatResponse,
    CommunityNodeBootstrapNode, CommunityNodeConsentItem, CommunityNodeConsentStatus,
    CommunityNodeResolvedUrls, CommunityNodeSeedPeer,
};
pub use normalize::{
    first_tag_value, normalize_http_url, normalize_http_url_list, normalize_pubkey,
    normalize_ws_url, parse_auth_envelope, parse_socket_addr_env, verify_auth_envelope,
};
pub use rendezvous::{
    TopicRendezvousCandidate, TopicRendezvousHeartbeat, TopicRendezvousHeartbeatResponse,
    TopicRendezvousStore, TopicRendezvousTopicResponse,
};
pub use reports::{
    COMMUNITY_NODE_REPORT_STATUS_RECEIVED, CommunityNodeReport, NewCommunityNodeReport,
    get_community_node_report, insert_community_node_report, list_community_node_reports,
};
pub use rollout::{ensure_default_auth_rollout, load_auth_rollout, store_auth_rollout};
pub use safety_events::{
    DistributionAudience, StoredModerationEvent, StoredRiskSignal, get_risk_signal,
    get_signed_moderation_event, list_distributable_moderation_events,
    list_distributable_risk_signals, list_risk_signals_for_target, list_signed_moderation_events,
    persist_risk_signal, persist_signed_moderation_event,
};
