ALTER TABLE cn_admin.policies
    ADD COLUMN IF NOT EXISTS effective_date DATE,
    ADD COLUMN IF NOT EXISTS language TEXT;

-- #860: 旧固定 placeholder への同意を実文書への同意として引き継がない。
DELETE FROM cn_user.policy_consents consent
USING cn_admin.policies policy
WHERE consent.policy_slug = policy.policy_slug
  AND (
    (policy.policy_slug = 'terms_of_service'
      AND policy.body_markdown = 'You must follow the community node terms of service.')
    OR
    (policy.policy_slug = 'privacy_policy'
      AND policy.body_markdown = 'You must acknowledge the community node privacy policy.')
  );
