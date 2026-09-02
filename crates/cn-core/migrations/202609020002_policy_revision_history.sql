-- #860 Phase B: current projection だった policies を公開済み正文の履歴表へ発展させる。
ALTER TABLE cn_user.policy_consents
    DROP CONSTRAINT IF EXISTS policy_consents_policy_slug_fkey;

ALTER TABLE cn_admin.policies
    DROP CONSTRAINT IF EXISTS policies_pkey;

ALTER TABLE cn_admin.policies
    ADD COLUMN IF NOT EXISTS is_current BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS retired_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS policy_snapshot_revision TEXT,
    ADD COLUMN IF NOT EXISTS material_change BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS requires_reconsent BOOLEAN NOT NULL DEFAULT FALSE,
    ADD CONSTRAINT policies_pkey PRIMARY KEY (policy_slug, policy_version);

CREATE UNIQUE INDEX IF NOT EXISTS idx_cn_admin_policies_one_current
    ON cn_admin.policies (policy_slug)
    WHERE is_current;

ALTER TABLE cn_user.policy_consents
    ADD COLUMN IF NOT EXISTS policy_snapshot_revision TEXT,
    ADD CONSTRAINT policy_consents_policy_revision_fkey
        FOREIGN KEY (policy_slug, policy_version)
        REFERENCES cn_admin.policies (policy_slug, policy_version);

CREATE TABLE IF NOT EXISTS cn_admin.policy_translations (
    policy_slug TEXT NOT NULL,
    policy_version INTEGER NOT NULL,
    language TEXT NOT NULL,
    translation_revision INTEGER NOT NULL,
    title TEXT NOT NULL,
    body_markdown TEXT NOT NULL,
    is_current BOOLEAN NOT NULL DEFAULT TRUE,
    retired_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (policy_slug, policy_version, language, translation_revision),
    FOREIGN KEY (policy_slug, policy_version)
        REFERENCES cn_admin.policies (policy_slug, policy_version)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_cn_admin_policy_translations_one_current
    ON cn_admin.policy_translations (policy_slug, policy_version, language)
    WHERE is_current;
