//! デスクトップが対向する community node の HTTP パス(cn endpoint contract の一部)。
//!
//! cn-user-api の route 定義と desktop-runtime の URL 構築が同じ定数を参照することで、
//! パス変更が両側にコンパイルエラーとして届く(文字列の二重持ちを解消。WP-H3 PR2)。

pub const AUTH_CHALLENGE_PATH: &str = "/v1/auth/challenge";
pub const AUTH_VERIFY_PATH: &str = "/v1/auth/verify";
pub const CONSENTS_PATH: &str = "/v1/consents";
pub const CONSENTS_STATUS_PATH: &str = "/v1/consents/status";
pub const BOOTSTRAP_NODES_PATH: &str = "/v1/bootstrap/nodes";
pub const BOOTSTRAP_HEARTBEAT_PATH: &str = "/v1/bootstrap/heartbeat";
pub const NODE_MANIFEST_PATH: &str = "/v1/node/manifest";
pub const TOPIC_RENDEZVOUS_HEARTBEAT_PATH: &str = "/v1/rendezvous/topics/heartbeat";
pub const INDEXING_REQUESTS_PATH: &str = "/v1/indexing/requests";
pub const INDEX_SEARCH_PATH: &str = "/v1/index/search";
pub const INDEX_DISCOVERY_PATH: &str = "/v1/index/discovery";
pub const INDEX_RECOMMENDATIONS_PATH: &str = "/v1/index/recommendations";
pub const TRUST_USERS_PATH_PREFIX: &str = "/v1/trust/users/";
pub const TRUST_USERS_ROUTE: &str = "/v1/trust/users/{pubkey}";
pub const RELATION_USERS_PATH_PREFIX: &str = "/v1/relation/users/";
pub const RELATION_USERS_ROUTE: &str = "/v1/relation/users/{target}";
pub const RELATION_NEIGHBORS_PATH: &str = "/v1/relation/neighbors";
pub const RELATION_OPTOUT_PATH: &str = "/v1/relation/optout";
/// 通報受付。client は manifest の `report_endpoint` から動的に解決するため
/// 直接この定数で URL を組み立てるのはサーバ側(route 定義)のみ。
pub const REPORT_PATH: &str = "/v1/report";
