CREATE SCHEMA IF NOT EXISTS cn_auth;
CREATE SCHEMA IF NOT EXISTS cn_user;
CREATE SCHEMA IF NOT EXISTS cn_admin;
CREATE SCHEMA IF NOT EXISTS cn_bootstrap;

CREATE TABLE IF NOT EXISTS cn_auth.auth_challenges (
    challenge TEXT PRIMARY KEY,
    pubkey TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cn_auth_challenges_pubkey
    ON cn_auth.auth_challenges (pubkey);

CREATE INDEX IF NOT EXISTS idx_cn_auth_challenges_expires_at
    ON cn_auth.auth_challenges (expires_at);

CREATE TABLE IF NOT EXISTS cn_user.subscriber_accounts (
    subscriber_pubkey TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_authenticated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_admin.policies (
    policy_slug TEXT PRIMARY KEY,
    policy_version INTEGER NOT NULL,
    title TEXT NOT NULL,
    body_markdown TEXT NOT NULL,
    required BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_user.policy_consents (
    subscriber_pubkey TEXT NOT NULL REFERENCES cn_user.subscriber_accounts (subscriber_pubkey) ON DELETE CASCADE,
    policy_slug TEXT NOT NULL REFERENCES cn_admin.policies (policy_slug) ON DELETE CASCADE,
    policy_version INTEGER NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (subscriber_pubkey, policy_slug, policy_version)
);

CREATE TABLE IF NOT EXISTS cn_admin.service_configs (
    service_name TEXT PRIMARY KEY,
    version BIGINT NOT NULL DEFAULT 1,
    config_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_bootstrap.bootstrap_nodes (
    base_url TEXT PRIMARY KEY,
    public_base_url TEXT NOT NULL,
    connectivity_urls JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
