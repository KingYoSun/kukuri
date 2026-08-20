//! コミュニティノード HTTP 境界の安定エラーコード(#712)。
//!
//! クライアントは機能の未提供・有効化の失効・利用拒否をこのコードで見分けて縮退する
//! (#670)。文字列はサーバ(`cn-user-api`)の発行、実行時層(`desktop-runtime`)の判別・
//! 後退処理、試験場面のスタブ(`harness`)が共有する通信契約であり、変更は互換性の検討と
//! セットで行うこと。通信境界の実値はサーバ側契約試験が `body.code` で固定する。
//!
//! TypeScript 側(`apps/desktop`)は生成型を持たないため文字列リテラルで判別する。
//! コード名を変更する場合はサーバ契約試験の失敗が検知装置になる。

/// 認証(bearer)が必要・無効(401)。
pub const AUTH_REQUIRED_CODE: &str = "AUTH_REQUIRED";

/// 必須ポリシーへの同意が未完了(403)。
pub const CONSENT_REQUIRED_CODE: &str = "CONSENT_REQUIRED";

/// このノードは索引参照を提供しない(未構成。404)。
pub const INDEX_QUERY_NOT_CONFIGURED_CODE: &str = "INDEX_QUERY_NOT_CONFIGURED";

/// 索引参照の有効化(準備完了記録)が失効している(404)。
pub const INDEX_QUERY_NOT_ACTIVATED_CODE: &str = "INDEX_QUERY_NOT_ACTIVATED";

/// このノードは信頼読み取りを提供しない(未構成。404)。
pub const TRUST_READ_NOT_CONFIGURED_CODE: &str = "TRUST_READ_NOT_CONFIGURED";

/// 信頼読み取りの有効化が失効している(404)。
pub const TRUST_READ_NOT_ACTIVATED_CODE: &str = "TRUST_READ_NOT_ACTIVATED";

/// 対象の関係観測が存在しない(404)。
pub const RELATION_NOT_FOUND_CODE: &str = "RELATION_NOT_FOUND";

/// このノードは距離利用停止(relation visibility)を提供しない(未構成。404)。
pub const RELATION_VISIBILITY_NOT_CONFIGURED_CODE: &str = "RELATION_VISIBILITY_NOT_CONFIGURED";

/// 距離利用停止の有効化が失効している(404)。
pub const RELATION_VISIBILITY_NOT_ACTIVATED_CODE: &str = "RELATION_VISIBILITY_NOT_ACTIVATED";
