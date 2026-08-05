-- #616: プロバイダ疎通確認の期限付き保存。
--
-- `cn-cli readiness` が外部プロバイダへの疎通確認（合成データのみ）の結果を保存し、
-- 有効期限内の再実行では外部プロバイダを叩かずにこの結果を使い回す（外部負荷と
-- 発報回数の抑制）。保存するのは判定と区分の要約のみで、資格情報の値・応答本文・
-- Match Data は構造的に持たない。

CREATE TABLE IF NOT EXISTS cn_admin.readiness_probe_cache (
    -- 疎通確認の対象 slot（known_csam / general / unknown_csam）。
    provider_slot TEXT PRIMARY KEY,
    -- slot に構成されたプロバイダ実装名（例: project-arachnid-shield）。
    provider TEXT NOT NULL,
    -- 判定（pass / fail）。
    status TEXT NOT NULL CHECK (status IN ('pass', 'fail')),
    -- 人間向けの要約（HTTP 状態の区分・時間切れ等。秘匿情報を含めない）。
    detail TEXT NOT NULL,
    -- 疎通確認を実行した時刻。
    checked_at TIMESTAMPTZ NOT NULL
);
