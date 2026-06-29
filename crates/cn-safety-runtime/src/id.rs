//! moderation event id の供給抽象と本番実装（#353 段階3b / #399）。
//!
//! 未署名 `ModerationEventBody` の `id` を生成する。orchestrator は id 生成方式に依存せず、
//! この trait を注入される。テストでは決定論的な連番 generator で event id を固定できる。
//!
//! 本番では [`UuidEventIdGenerator`] を `Arc<dyn EventIdGenerator>` として注入し、UUID v4 を
//! moderation event id として使う。

use uuid::Uuid;

/// moderation event の一意 id を生成する抽象。
pub trait EventIdGenerator: Send + Sync {
    /// 新しい event id を返す。
    fn next_id(&self) -> String;
}

/// UUID v4 ベースの本番 [`EventIdGenerator`] 実装。
///
/// `next_id()` はハイフン付き小文字の UUID v4 文字列を返す
/// （例: `550e8400-e29b-41d4-a716-446655440000`）。乱数ベースのため event id は
/// プロセス / ノードをまたいで衝突しない前提で扱える。
#[derive(Clone, Copy, Debug, Default)]
pub struct UuidEventIdGenerator;

impl UuidEventIdGenerator {
    /// 新しい `UuidEventIdGenerator` を作る。
    pub fn new() -> Self {
        Self
    }
}

impl EventIdGenerator for UuidEventIdGenerator {
    fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }
}
