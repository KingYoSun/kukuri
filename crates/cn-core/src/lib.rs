mod auth;
mod bootstrap;
mod config;
mod consents;
mod database;
mod errors;
mod models;
mod normalize;
mod rendezvous;
mod reports;
mod rollout;
#[cfg(test)]
mod tests;

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
    AuthRolloutConfig, BOOTSTRAP_PEER_REGISTRATION_TTL_SECONDS, COMMUNITY_NODE_AUTH_SERVICE_NAME,
    COMMUNITY_NODE_DATABASE_INIT_MODE_ENV, COMMUNITY_NODE_RENDEZVOUS_KEY_PREFIX_ENV,
    COMMUNITY_NODE_RENDEZVOUS_REDIS_URL_ENV, DEFAULT_TOKEN_TTL_SECONDS, DatabaseInitMode,
    JwtConfig, TOPIC_RENDEZVOUS_TTL_SECONDS, USER_API_BEARER_CHALLENGE,
};
pub use consents::{accept_consents, get_consent_status, require_consents};
pub use database::{
    TestDatabase, connect_postgres, ensure_database_ready, initialize_database,
    initialize_database_for_runtime, migrate_postgres, seed_default_policies,
};
pub use errors::{ApiError, ApiResult, auth_required_error, consent_required_error};
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
