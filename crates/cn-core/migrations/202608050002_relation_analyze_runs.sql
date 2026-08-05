-- #616: 関係解析（cn-cli relation analyze）の実行記録。
--
-- 定期実行の最終成否を readiness（relation_analysis_recent）が機械判定するための記録。
-- 失敗時の error はエラー種別の要約のみで、秘匿情報・応答本文を含めない（書き込み側の契約）。

CREATE TABLE IF NOT EXISTS cn_admin.relation_analyze_runs (
    id BIGSERIAL PRIMARY KEY,
    started_at TIMESTAMPTZ NOT NULL,
    finished_at TIMESTAMPTZ NOT NULL,
    success BOOLEAN NOT NULL,
    edges_upserted BIGINT NOT NULL DEFAULT 0,
    clusters_assigned BIGINT NOT NULL DEFAULT 0,
    error TEXT
);

CREATE INDEX IF NOT EXISTS idx_cn_admin_relation_analyze_runs_finished_at
    ON cn_admin.relation_analyze_runs (finished_at DESC);
