//! 取り込み 1 件の失敗の分類（#1090）。
//!
//! 失敗は次の二つに分ける。印の無い失敗は確定した理由として扱う（従来どおり de-index する）。
//!   - 確定した理由: state / envelope の破損・署名不一致、参照の変化、撤回、送信防止、
//!     scope 非対応、本文・manifest の検証失敗。既存 entry を真実源・投影から消す。
//!   - 一時的な失敗: replica の照会、本文 blob の一時取得、真実源・投影・判定記録の読み書きの障害。
//!     既存 entry は保持し、この走査では新たに索引しない（次の走査で再評価する）。
//!
//! 一時的な失敗は投稿の内容を変えない。object id は署名済み envelope に束縛されるため、保持した
//! entry の本文は索引時に検証した署名済み内容のままであり、撤回・削除・送信防止などの確定した
//! 理由は次の走査で従来どおり評価される。

use std::fmt;

/// 一時的な失敗の印（`anyhow::Error` の context として付ける）。
#[derive(Debug)]
pub(super) struct Transient;

impl fmt::Display for Transient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("temporary ingest failure")
    }
}

/// 失敗に一時的な失敗の印を付ける。
pub(super) fn transient(error: anyhow::Error) -> anyhow::Error {
    error.context(Transient)
}

/// 一時的な失敗の印があるか（外側に別の context が重なっていても判定できる）。
pub(super) fn is_transient(error: &anyhow::Error) -> bool {
    error.downcast_ref::<Transient>().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, anyhow};

    #[test]
    fn marker_survives_outer_context_and_is_absent_by_default() {
        let plain = anyhow!("post state changed during scan");
        assert!(!is_transient(&plain));

        let marked = transient(anyhow!("replica query failed"));
        assert!(is_transient(&marked));
        let wrapped = Err::<(), _>(marked)
            .context("failed to ingest object")
            .unwrap_err();
        assert!(is_transient(&wrapped));
        assert!(format!("{wrapped:#}").contains("replica query failed"));
    }
}
