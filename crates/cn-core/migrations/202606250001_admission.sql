-- #383: community node の admission（招待コード / whitelist / ban）。
--
-- public community node の利用者を限定するための server-side enforcement state。
-- これは node-local な「補助機能提供の可否」判断であり、kukuri network 全体からの
-- アカウント凍結ではない（docs/architecture/p2p-first-community-node-responsibility-boundary.md）。

-- 招待コード。平文は保存せず、SHA-256 hash のみを保持する。
CREATE TABLE IF NOT EXISTS cn_admin.invite_codes (
    -- 招待コード平文の SHA-256 hex digest。
    code_hash TEXT PRIMARY KEY,
    -- 運営者向けの任意ラベル。
    label TEXT,
    -- 使用可能回数。NULL = 無制限、1 = 単回。
    max_uses INTEGER,
    -- これまでの使用回数。
    used_count INTEGER NOT NULL DEFAULT 0,
    -- 失効時刻。NULL = 無期限。
    expires_at TIMESTAMPTZ,
    -- 取り消し時刻。非 NULL なら使用不可。
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 手動許可（whitelist）。pubkey 単位。
CREATE TABLE IF NOT EXISTS cn_admin.admission_allowlist (
    pubkey TEXT PRIMARY KEY,
    label TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- admission を実際に通過した（=この node の member になった）ことを表すフラグ。
--
-- これにより「現メンバー（mode 変更後も再認証を bypass できる）」と、
-- 「未参加のまま事前 ban され、後で unban されただけの pubkey（再び invite/whitelist が必要）」を
-- 区別する。status='active' だけでは両者を区別できず、unban が invite/whitelist を迂回してしまう。
ALTER TABLE cn_user.subscriber_accounts
    ADD COLUMN IF NOT EXISTS admitted BOOLEAN NOT NULL DEFAULT FALSE;

-- 既存 subscriber は open-only model 下で admission 済みなので member として backfill する。
UPDATE cn_user.subscriber_accounts
SET admitted = TRUE
WHERE status = 'active';
