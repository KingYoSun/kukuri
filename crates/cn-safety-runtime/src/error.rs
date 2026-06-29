//! orchestrator 構築時のエラー（#353 段階3b）。
//!
//! scan 実行時の provider 失敗は `ScanError` を `ScanOutcome` に写像して fail-closed な
//! verdict に集約するため、`scan_subject` は `Result` を返さない。ここで定義するエラーは
//! orchestrator を **構築する時点** の構成不備のみを表す。

use thiserror::Error;

/// `SafetyOrchestrator` 構築時の構成エラー。
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SafetyRuntimeError {
    /// provider が capability を 1 つも宣言していない。
    ///
    /// provider が `Err` を返したとき、その結果を `ProviderScanResult` に写像するために
    /// capability が必要になる。capability の無い provider は scan 失敗時に結果を合成できず
    /// fail-closed の集約から漏れるため、構築時に拒否する。
    #[error("safety provider '{provider}' declares no capabilities")]
    ProviderWithoutCapability { provider: String },

    /// provider 名が空。監査・verdict 反映で識別子が必要なため拒否する。
    #[error("safety provider name must not be empty")]
    EmptyProviderName,

    /// issuer node id が空。moderation event の発行者として必須。
    #[error("issuer_node_id must not be empty")]
    EmptyIssuerNodeId,

    /// provider が 1 つも登録されていない。
    ///
    /// provider が無いと scan 結果が空になり、route() は unscanned として fail-closed に倒すが、
    /// 構成ミスを早期に検出するため構築時に拒否する。
    #[error("at least one safety provider must be registered")]
    NoProviders,
}
