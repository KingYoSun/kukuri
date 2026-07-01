-- #413: community node ingestion（Model C）の scope 管理 state。
--
-- indexing = Model C（docs replica sync participant）は operator が引き受けた supported topic /
-- 許可 channel の共有 replica のみを ingest する（ADR 0025 §2.2 / §6）。本 migration はその
-- node-local な運用 state を保持する:
--   - supported_topics: operator が index を引き受けた public topic / private channel の集合。
--   - indexing_requests: user からの indexing 要求（request → operator 承認 → index の多段ゲート）。
--   - channel_secrets: private channel の capability（namespace secret）。at-rest 暗号化して保存する。
--
-- index 投影本体（検索対象データ）は ArcadeDB（canonical ではない写像）に置き、ここには置かない。
-- これらは canonical source ではなく、いつでも再構築できる node-local state（ADR 0025 §2.1）。
CREATE SCHEMA IF NOT EXISTS cn_index;

-- operator が index を引き受けた scope。
--
-- kind は `public_topic`（`topic::<id>` を導出 namespace で open）と `private_channel`
-- （`channel::<id>` を登録 capability で open）を区別する。id は topic_id / channel_id。
CREATE TABLE cn_index.supported_topics (
    -- topic_id（public_topic）または channel_id（private_channel）。
    id TEXT NOT NULL,
    -- ingest scope の種別（public_topic / private_channel）。
    kind TEXT NOT NULL,
    -- supported set へ追加した時刻。
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (kind, id)
);

-- kind ごとの列挙（起動時の replica open で使う）。
CREATE INDEX idx_cn_index_supported_topics_kind ON cn_index.supported_topics (kind);

-- user からの indexing request。
--
-- request は index を保証しない。operator が承認（supported 化）し、さらに safety verdict を
-- 通過した content のみが index される（ADR 0025 §2.2）。同一 requester の同一対象への再要求は
-- 冪等に更新する。
CREATE TABLE cn_index.indexing_requests (
    -- request id（UUID v4）。
    id TEXT PRIMARY KEY,
    -- 要求者の公開鍵（認証済み bearer の pubkey）。
    requester_pubkey TEXT NOT NULL,
    -- 対象 scope の種別（public_topic / private_channel）。
    kind TEXT NOT NULL,
    -- 対象識別子（topic_id / channel_id）。
    target_id TEXT NOT NULL,
    -- 処理状態（pending / approved / rejected）。
    status TEXT NOT NULL DEFAULT 'pending',
    -- 要求時刻。
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- operator が承認 / 却下した時刻（未処理は NULL）。
    decided_at TIMESTAMPTZ,
    -- 同一 requester の同一対象への重複要求を冪等化する。
    UNIQUE (kind, target_id, requester_pubkey)
);

-- operator の未処理一覧（新着順）と status 絞り込み。
CREATE INDEX idx_cn_index_indexing_requests_status_created_at
    ON cn_index.indexing_requests (status, created_at DESC);

-- private channel の capability（namespace secret）。
--
-- indexing リクエスト＝secret 送信（ADR 0025 §6.3）。secret を提示できること自体を channel 権限の
-- 証明とみなし、CN は新しい権限体系を作らない。平文は列に残さず、nonce + ciphertext（XChaCha20Poly1305）
-- のみを保持する。復号鍵は runtime（Secret Manager / env 注入）が供給し、DB には置かない。
CREATE TABLE cn_index.channel_secrets (
    -- channel_id（`channel::<id>` の id 部）。
    channel_id TEXT PRIMARY KEY,
    -- XChaCha20Poly1305 nonce（24 bytes）。
    nonce BYTEA NOT NULL,
    -- 暗号化された namespace secret hex。
    ciphertext BYTEA NOT NULL,
    -- 登録 / 更新時刻。
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
