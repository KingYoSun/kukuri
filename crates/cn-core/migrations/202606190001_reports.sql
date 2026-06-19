-- #370: community node の通報受信（report intake）。
--
-- node は自分の authority scope 内（自分が index / moderate / cache / relay / recommend した対象）
-- に対する通報のみ受理する。中央通報窓口ではない。reporter の identity / social graph は保持せず、
-- 明示的に入力された連絡先（任意）のみ保存する。
CREATE TABLE cn_admin.reports (
    id TEXT PRIMARY KEY,
    -- 通報対象の種別（post / profile / media / search_result / recommendation 等）。
    subject_kind TEXT NOT NULL,
    -- 通報対象の識別子（post id / pubkey / object id 等）。
    subject_id TEXT NOT NULL,
    -- 通報先となった node capability（community_index / moderation / media_cache 等）。
    capability TEXT NOT NULL,
    -- 通報理由カテゴリ。
    reason TEXT NOT NULL,
    -- 任意の補足説明。
    details TEXT,
    -- 任意の通報者連絡先（node が follow-up に使える）。identity ではなく入力値のみ。
    reporter_contact TEXT,
    -- 処理状態（received / reviewing / actioned / dismissed 等）。
    status TEXT NOT NULL DEFAULT 'received',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 運営者の一覧表示（新着順）。
CREATE INDEX idx_cn_admin_reports_created_at ON cn_admin.reports (created_at DESC);

-- 対象ごとの集約・参照。
CREATE INDEX idx_cn_admin_reports_subject ON cn_admin.reports (subject_kind, subject_id);

-- 状態での絞り込み。
CREATE INDEX idx_cn_admin_reports_status ON cn_admin.reports (status);
