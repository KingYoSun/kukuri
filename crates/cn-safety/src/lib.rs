//! kukuri community node 向け CSAM / critical safety moderation の **pure domain 層**（#353 段階2）。
//!
//! この crate は DB / network / production provider credentials に依存しない。
//! safety domain model・provider abstraction・mock provider・policy router を提供し、
//! deterministic な単体テストで verdict 分岐と fail-closed 挙動を固定する。
//!
//! 設計の真実源:
//! - `docs/adr/0027-deterministic-moderation-critical-safety.md`
//!   - §2.2 verdict model（action と label の分離、`csam_confirmed` と `csam_suspected` の区別）
//!   - §2.4 fail-closed invariants（unscanned / scan failure / provider unavailable を allow にしない）
//!   - §2.7 provider abstraction / §2.3 policy routing
//!   - §2.1 advisory ≠ command（moderation event / risk signal は issuer node の authority scope 内の
//!     advisory であり network-wide command ではない）
//!   - §2.6 visibility（local / subscribed_nodes / public の3段階、suspected unknown CSAM / CSE は既定 `local`）
//!
//! スコープ境界（本 crate に含まないもの）:
//! - public-node readiness check / `safety` CLI（段階3）
//! - 本番 provider 接続（#391 Project Arachnid Shield 等）
//! - fail-closed indexing 本体（search / discovery / recommendation 除外の DB 制約）
//! - moderation event の実鍵署名（secp256k1）と P2P 配布
//!
//! serde 表現は既存 community-node crate（`cn-core` / `desktop-runtime`）に合わせて
//! すべて `snake_case`。client（TS）向けの wire 変換が必要になった段階で別途変換層を入れる。

pub mod capability;
pub mod event;
pub mod policy;
pub mod provider;
pub mod signal;
pub mod verdict;

/// deterministic な mock provider / signer。`mock` feature でのみ有効。
///
/// `MockSigner` は非暗号（FNV-1a）であり本番経路で署名として誤用しないよう、production の
/// 既定 API には含めない。テスト / 開発では `mock` feature を有効にして使う。
#[cfg(feature = "mock")]
pub mod mock;

pub use capability::SafetyProviderCapability;
pub use event::{
    ModerationEventBody, ModerationEventSigner, SignedModerationEvent, issue_signed_event,
};
#[cfg(feature = "mock")]
pub use mock::{MockSafetyProvider, MockSigner};
pub use policy::{SafetyPolicy, route};
pub use provider::{
    ProviderScanRequest, ProviderScanResult, SafetyProvider, ScanError, ScanOutcome, SubjectKind,
};
pub use signal::{AppealStatus, RiskSignalTarget, SafetyRiskSignal};
pub use verdict::{
    Basis, ModerationAction, ReasonCode, SafetyAction, SafetyCategory, SafetyLabel, SafetyVerdict,
    Severity, Visibility,
};
