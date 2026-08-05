//! 観測用の実行状態（#613 T3）。
//!
//! 常駐ワーカーが更新し、テスト・起動完了判定（#612）・HTTP 状態エンドポイント（`status`）が
//! 読む共有状態。観測用の状態の置き場はここだけにする（他の場所に分散させない）。
//!
//! 時刻はすべて呼び出し側が unix 秒で渡す（この構造体は時計を持たない）。

use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::ingest::IngestSummary;

/// 観測状態の写し。`GET /v1/status` はこの形をそのまま JSON で返す。
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct IndexerStateSnapshot {
    /// ワーカーが動いているか。
    pub worker_running: bool,
    /// 取り込みが有効か（安全性プロバイダ未設定なら false のまま常駐する）。
    pub ingest_enabled: bool,
    /// 開いているスコープ数。
    pub opened_scopes: u64,
    /// 最後に全件見直し（restore → 取り込み 1 巡）が成功した時刻（unix 秒）。
    pub last_sync_at: Option<i64>,
    /// 最後にスコープ取り込みが成功した時刻（unix 秒）。
    pub last_ingest_at: Option<i64>,
    /// 最後のエラー内容。
    pub last_error: Option<String>,
    /// 最後のエラーが起きた対象スコープ（`replica id` 表現。全体エラーなら None）。
    pub last_error_scope: Option<String>,
    /// 走査した項目数の累計。
    pub scanned: u64,
    /// 許可されて索引に入れた件数の累計。
    pub indexed: u64,
    /// 不許可などで索引に入れなかった件数の累計（下の 2 つの分類を含む）。
    pub skipped_non_allow: u64,
    /// スキャン失敗（scan_failed / unscanned / スキャン呼び出しの失敗）の件数。
    pub scan_errors: u64,
    /// プロバイダ利用不可の件数。
    pub provider_unavailable: u64,
    /// 索引から外した件数の累計。
    pub deindexed: u64,
    /// メディア取得の成功件数。
    pub media_fetch_success: u64,
    /// メディア取得の利用不可（未複製・ピア不在など）件数。
    pub media_fetch_unavailable: u64,
    /// メディア取得の時間切れ件数。
    pub media_fetch_timeout: u64,
    /// メディア取得の大きさ超過件数。
    pub media_fetch_oversize: u64,
}

/// 共有の観測状態。ワーカー・取り込みパイプライン・メディア取得器が更新する。
#[derive(Debug, Default)]
pub struct IndexerRuntimeState {
    worker_running: AtomicBool,
    ingest_enabled: AtomicBool,
    opened_scopes: AtomicU64,
    last_sync_at: RwLock<Option<i64>>,
    last_ingest_at: RwLock<Option<i64>>,
    last_error: RwLock<Option<(String, Option<String>)>>,
    scanned: AtomicU64,
    indexed: AtomicU64,
    skipped_non_allow: AtomicU64,
    scan_errors: AtomicU64,
    provider_unavailable: AtomicU64,
    deindexed: AtomicU64,
    media_fetch_success: AtomicU64,
    media_fetch_unavailable: AtomicU64,
    media_fetch_timeout: AtomicU64,
    media_fetch_oversize: AtomicU64,
}

impl IndexerRuntimeState {
    pub fn set_worker_running(&self, running: bool) {
        self.worker_running.store(running, Ordering::Relaxed);
    }

    pub fn set_ingest_enabled(&self, enabled: bool) {
        self.ingest_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_opened_scopes(&self, count: u64) {
        self.opened_scopes.store(count, Ordering::Relaxed);
    }

    /// 全件見直しの成功を記録する（unix 秒）。
    pub fn record_sync_success(&self, at_unix: i64) {
        *self.last_sync_at.write().expect("last_sync_at poisoned") = Some(at_unix);
    }

    /// スコープ取り込みの成功と、その取り込み結果の件数を記録する。
    pub fn record_ingest_success(&self, at_unix: i64, summary: &IngestSummary) {
        *self
            .last_ingest_at
            .write()
            .expect("last_ingest_at poisoned") = Some(at_unix);
        self.scanned
            .fetch_add(summary.scanned as u64, Ordering::Relaxed);
        self.indexed
            .fetch_add(summary.indexed as u64, Ordering::Relaxed);
        self.skipped_non_allow
            .fetch_add(summary.skipped_non_allow as u64, Ordering::Relaxed);
        self.deindexed
            .fetch_add(summary.deindexed as u64, Ordering::Relaxed);
    }

    /// エラーを記録する（scope は replica id 表現。全体エラーなら None）。
    pub fn record_error(&self, scope: Option<&str>, error: &str) {
        *self.last_error.write().expect("last_error poisoned") =
            Some((error.to_string(), scope.map(str::to_string)));
    }

    /// 索引解除（スコープ単位の削除など、取り込み結果の外で行った分）を記録する。
    pub fn record_deindexed(&self, count: u64) {
        self.deindexed.fetch_add(count, Ordering::Relaxed);
    }

    pub fn record_scan_error(&self) {
        self.scan_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_provider_unavailable(&self) {
        self.provider_unavailable.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_media_fetch_success(&self) {
        self.media_fetch_success.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_media_fetch_unavailable(&self) {
        self.media_fetch_unavailable.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_media_fetch_timeout(&self) {
        self.media_fetch_timeout.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_media_fetch_oversize(&self) {
        self.media_fetch_oversize.fetch_add(1, Ordering::Relaxed);
    }

    /// 現在の状態の写しを返す。
    pub fn snapshot(&self) -> IndexerStateSnapshot {
        let (last_error, last_error_scope) =
            match self.last_error.read().expect("last_error poisoned").clone() {
                Some((error, scope)) => (Some(error), scope),
                None => (None, None),
            };
        IndexerStateSnapshot {
            worker_running: self.worker_running.load(Ordering::Relaxed),
            ingest_enabled: self.ingest_enabled.load(Ordering::Relaxed),
            opened_scopes: self.opened_scopes.load(Ordering::Relaxed),
            last_sync_at: *self.last_sync_at.read().expect("last_sync_at poisoned"),
            last_ingest_at: *self.last_ingest_at.read().expect("last_ingest_at poisoned"),
            last_error,
            last_error_scope,
            scanned: self.scanned.load(Ordering::Relaxed),
            indexed: self.indexed.load(Ordering::Relaxed),
            skipped_non_allow: self.skipped_non_allow.load(Ordering::Relaxed),
            scan_errors: self.scan_errors.load(Ordering::Relaxed),
            provider_unavailable: self.provider_unavailable.load(Ordering::Relaxed),
            deindexed: self.deindexed.load(Ordering::Relaxed),
            media_fetch_success: self.media_fetch_success.load(Ordering::Relaxed),
            media_fetch_unavailable: self.media_fetch_unavailable.load(Ordering::Relaxed),
            media_fetch_timeout: self.media_fetch_timeout.load(Ordering::Relaxed),
            media_fetch_oversize: self.media_fetch_oversize.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reflects_flags_and_counters() {
        let state = IndexerRuntimeState::default();
        assert_eq!(state.snapshot(), IndexerStateSnapshot::default());

        state.set_worker_running(true);
        state.set_ingest_enabled(true);
        state.set_opened_scopes(2);
        state.record_sync_success(100);
        state.record_ingest_success(
            101,
            &IngestSummary {
                scanned: 3,
                indexed: 2,
                skipped_non_allow: 1,
                deindexed: 0,
            },
        );
        state.record_scan_error();
        state.record_provider_unavailable();
        state.record_deindexed(4);
        state.record_media_fetch_success();
        state.record_media_fetch_unavailable();
        state.record_media_fetch_timeout();
        state.record_media_fetch_oversize();
        state.record_error(Some("topic::rust"), "boom");

        let snapshot = state.snapshot();
        assert!(snapshot.worker_running);
        assert!(snapshot.ingest_enabled);
        assert_eq!(snapshot.opened_scopes, 2);
        assert_eq!(snapshot.last_sync_at, Some(100));
        assert_eq!(snapshot.last_ingest_at, Some(101));
        assert_eq!(snapshot.last_error.as_deref(), Some("boom"));
        assert_eq!(snapshot.last_error_scope.as_deref(), Some("topic::rust"));
        assert_eq!(snapshot.scanned, 3);
        assert_eq!(snapshot.indexed, 2);
        assert_eq!(snapshot.skipped_non_allow, 1);
        assert_eq!(snapshot.scan_errors, 1);
        assert_eq!(snapshot.provider_unavailable, 1);
        assert_eq!(snapshot.deindexed, 4);
        assert_eq!(snapshot.media_fetch_success, 1);
        assert_eq!(snapshot.media_fetch_unavailable, 1);
        assert_eq!(snapshot.media_fetch_timeout, 1);
        assert_eq!(snapshot.media_fetch_oversize, 1);
    }

    #[test]
    fn snapshot_serializes_to_json_with_stable_field_names() {
        let state = IndexerRuntimeState::default();
        let json = serde_json::to_value(state.snapshot()).expect("serialize");
        // 起動完了判定（#612）が機械的に読む代表フィールド名を固定する。
        assert!(json.get("worker_running").is_some());
        assert!(json.get("ingest_enabled").is_some());
        assert!(json.get("opened_scopes").is_some());
        assert!(json.get("last_sync_at").is_some());
        assert!(json.get("provider_unavailable").is_some());
        assert!(json.get("media_fetch_timeout").is_some());
    }
}
