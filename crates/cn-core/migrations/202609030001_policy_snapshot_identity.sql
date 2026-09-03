-- #860 Phase B rerun: policy snapshot を公開済み正文と同意の immutable identity にする。
-- 表示用 version が同じでも法務 snapshot が変われば別 revision として保持する。

ALTER TABLE cn_user.policy_consents
    DROP CONSTRAINT IF EXISTS policy_consents_policy_revision_fkey;

ALTER TABLE cn_admin.policy_translations
    DROP CONSTRAINT IF EXISTS policy_translations_policy_slug_policy_version_fkey;

UPDATE cn_admin.policies
SET policy_snapshot_revision = 'legacy:' || md5('default-policy-catalog-v1')
WHERE policy_snapshot_revision IS NULL OR btrim(policy_snapshot_revision) = '';

ALTER TABLE cn_admin.policies
    ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ;

UPDATE cn_admin.policies
SET published_at = updated_at
WHERE published_at IS NULL;

ALTER TABLE cn_admin.policies
    ALTER COLUMN published_at SET DEFAULT NOW(),
    ALTER COLUMN published_at SET NOT NULL,
    ALTER COLUMN policy_snapshot_revision SET NOT NULL;

ALTER TABLE cn_admin.policies
    ADD COLUMN IF NOT EXISTS predecessor_policy_version INTEGER,
    ADD COLUMN IF NOT EXISTS predecessor_snapshot_revision TEXT;

WITH ordered AS (
    SELECT
        policy_slug,
        policy_version,
        policy_snapshot_revision,
        LAG(policy_version) OVER (
            PARTITION BY policy_slug ORDER BY policy_version, published_at, policy_snapshot_revision
        ) AS predecessor_policy_version,
        LAG(policy_snapshot_revision) OVER (
            PARTITION BY policy_slug ORDER BY policy_version, published_at, policy_snapshot_revision
        ) AS predecessor_snapshot_revision
    FROM cn_admin.policies
)
UPDATE cn_admin.policies policy
SET predecessor_policy_version = ordered.predecessor_policy_version,
    predecessor_snapshot_revision = ordered.predecessor_snapshot_revision
FROM ordered
WHERE policy.policy_slug = ordered.policy_slug
  AND policy.policy_version = ordered.policy_version
  AND policy.policy_snapshot_revision = ordered.policy_snapshot_revision;

ALTER TABLE cn_user.policy_consents
    ADD COLUMN IF NOT EXISTS policy_snapshot_revision TEXT;

UPDATE cn_user.policy_consents consent
SET policy_snapshot_revision = policy.policy_snapshot_revision
FROM cn_admin.policies policy
WHERE consent.policy_slug = policy.policy_slug
  AND consent.policy_version = policy.policy_version
  AND (consent.policy_snapshot_revision IS NULL OR btrim(consent.policy_snapshot_revision) = '');

ALTER TABLE cn_user.policy_consents
    ALTER COLUMN policy_snapshot_revision SET NOT NULL;

ALTER TABLE cn_admin.policy_translations
    ADD COLUMN IF NOT EXISTS policy_snapshot_revision TEXT;

UPDATE cn_admin.policy_translations translation
SET policy_snapshot_revision = policy.policy_snapshot_revision
FROM cn_admin.policies policy
WHERE translation.policy_slug = policy.policy_slug
  AND translation.policy_version = policy.policy_version
  AND (translation.policy_snapshot_revision IS NULL OR btrim(translation.policy_snapshot_revision) = '');

ALTER TABLE cn_admin.policy_translations
    ALTER COLUMN policy_snapshot_revision SET NOT NULL;

ALTER TABLE cn_user.policy_consents
    DROP CONSTRAINT IF EXISTS policy_consents_pkey;
ALTER TABLE cn_admin.policy_translations
    DROP CONSTRAINT IF EXISTS policy_translations_pkey;
ALTER TABLE cn_admin.policies
    DROP CONSTRAINT IF EXISTS policies_pkey;

ALTER TABLE cn_admin.policies
    ADD CONSTRAINT policies_pkey PRIMARY KEY (
        policy_slug, policy_version, policy_snapshot_revision
    );

ALTER TABLE cn_user.policy_consents
    ADD CONSTRAINT policy_consents_pkey PRIMARY KEY (
        subscriber_pubkey, policy_slug, policy_version, policy_snapshot_revision
    ),
    ADD CONSTRAINT policy_consents_policy_revision_fkey
        FOREIGN KEY (policy_slug, policy_version, policy_snapshot_revision)
        REFERENCES cn_admin.policies (
            policy_slug, policy_version, policy_snapshot_revision
        );

ALTER TABLE cn_admin.policy_translations
    ADD CONSTRAINT policy_translations_pkey PRIMARY KEY (
        policy_slug, policy_version, policy_snapshot_revision, language, translation_revision
    ),
    ADD CONSTRAINT policy_translations_policy_revision_fkey
        FOREIGN KEY (policy_slug, policy_version, policy_snapshot_revision)
        REFERENCES cn_admin.policies (
            policy_slug, policy_version, policy_snapshot_revision
        );

ALTER TABLE cn_admin.policies
    ADD CONSTRAINT policies_predecessor_revision_fkey
        FOREIGN KEY (
            policy_slug, predecessor_policy_version, predecessor_snapshot_revision
        )
        REFERENCES cn_admin.policies (
            policy_slug, policy_version, policy_snapshot_revision
        );

DROP INDEX IF EXISTS cn_admin.idx_cn_admin_policy_translations_one_current;
CREATE UNIQUE INDEX idx_cn_admin_policy_translations_one_current
    ON cn_admin.policy_translations (
        policy_slug, policy_version, policy_snapshot_revision, language
    )
    WHERE is_current;

CREATE INDEX idx_cn_admin_policies_snapshot_lookup
    ON cn_admin.policies (policy_slug, policy_snapshot_revision);
