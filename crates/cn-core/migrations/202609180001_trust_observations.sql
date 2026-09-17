-- #1061: ブロック / ミュート観測（ADR 0026 §8.3）。
--
-- observer 本人が署名した観測を (observer, target, kind) 単位で最新の 1 件だけ保持する。
-- revision は対象ごとの relation_version（§8.4）に使い、観測の追加・解除・削除のたびに進める。
-- 観測は node-local で、cross-node pull には返さない。

CREATE TABLE IF NOT EXISTS cn_trust.observations (
    observer_pubkey TEXT NOT NULL,
    target_pubkey TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('block', 'mute')),
    active BOOLEAN NOT NULL,
    observed_at TIMESTAMPTZ NOT NULL,
    observed_at_ms BIGINT NOT NULL,
    envelope_id TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (observer_pubkey, target_pubkey, kind),
    CHECK (observer_pubkey <> target_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_cn_trust_observations_active_target
    ON cn_trust.observations (target_pubkey, observed_at DESC)
    WHERE active;

-- 対象ごとの観測 revision。観測行の削除でも巻き戻らないよう別表で単調に進める。
CREATE SEQUENCE IF NOT EXISTS cn_trust.observation_revision_seq;

CREATE TABLE IF NOT EXISTS cn_trust.observation_target_revisions (
    target_pubkey TEXT PRIMARY KEY,
    revision BIGINT NOT NULL
);

-- 任意文書 trust_observation_sharing への同意の取消時刻。取消より後の同意だけを有効とする。
CREATE TABLE IF NOT EXISTS cn_trust.observation_sharing_revocations (
    observer_pubkey TEXT PRIMARY KEY,
    revoked_at TIMESTAMPTZ NOT NULL
);
