//! moderation event id の供給抽象（#353 段階3b）。
//!
//! 未署名 `ModerationEventBody` の `id` を生成する。orchestrator は id 生成方式に依存せず、
//! この trait を注入される。テストでは決定論的な連番 generator で event id を固定できる。
//!
//! 本番の id 実装（UUID / ULID）は本 crate のスコープ外であり、runtime 組み込み段階で別途
//! 追加する（別 Issue 化済み）。

/// moderation event の一意 id を生成する抽象。
pub trait EventIdGenerator: Send + Sync {
    /// 新しい event id を返す。
    fn next_id(&self) -> String;
}
