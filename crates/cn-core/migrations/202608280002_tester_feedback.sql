-- #802 / ADR 0039: テスターフィードバックの受付・蓄積。
--
-- テスターの自由記述 3 項目と自動付与された client version / OS のみを保存する。
-- 送信者の identity(pubkey)は保存しない。一般通報(cn_admin.reports)とは
-- 必要情報・保存方針・閲覧目的が異なるため record を分離する。
CREATE TABLE cn_admin.tester_feedback (
    id TEXT PRIMARY KEY,
    -- やろうとしたこと。
    what_attempted TEXT NOT NULL,
    -- 何が起きたか。
    what_happened TEXT NOT NULL,
    -- 何が変だと思ったか。
    what_seemed_wrong TEXT NOT NULL,
    -- 送信元 client のバージョン(client が自動付与)。
    client_version TEXT NOT NULL,
    -- 送信元 client の OS(client が自動付与)。
    os TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 区分別保持(retention.tester_feedback_days)の絶対期限。
    expires_at TIMESTAMPTZ NOT NULL
);

-- 運営者の一覧表示(新着順)。
CREATE INDEX idx_cn_admin_tester_feedback_created_at
    ON cn_admin.tester_feedback (created_at DESC);
