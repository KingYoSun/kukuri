-- #616: 読み取り面の有効化記録。
--
-- `cn-cli readiness` が全項目合格を確認したときにのみ行を追加する。cn-user-api は
-- 環境変数（COMMUNITY_NODE_INDEX_QUERY_ENABLED / COMMUNITY_NODE_TRUST_READ_ENABLED）が
-- 真でも、有効な記録が無ければ該当の読み取り面を公開しない（有効化の関門）。
-- report は判定の id / 状態 / 要約のみで、資格情報・応答本文を含めない（書き込み側の契約）。

CREATE TABLE IF NOT EXISTS cn_admin.readiness_activations (
    id BIGSERIAL PRIMARY KEY,
    activated_at TIMESTAMPTZ NOT NULL,
    profile TEXT NOT NULL,
    -- 評価時点の判定項目 id の配列。cn-user-api は現行の判定項目集合との一致を検証し、
    -- 集合が変わった（= 判定基準が変わった）記録を無効として扱う。
    check_ids JSONB NOT NULL,
    -- 全合格の報告本文（id / status / detail の配列）。監査用。
    report JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cn_admin_readiness_activations_activated_at
    ON cn_admin.readiness_activations (activated_at DESC);
