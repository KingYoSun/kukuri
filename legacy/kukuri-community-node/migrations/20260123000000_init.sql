CREATE SCHEMA IF NOT EXISTS cn_admin;
CREATE SCHEMA IF NOT EXISTS cn_user;

CREATE TABLE IF NOT EXISTS cn_admin.admin_users (
    admin_user_id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_admin.admin_sessions (
    session_id TEXT PRIMARY KEY,
    admin_user_id TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT admin_sessions_user_fk
        FOREIGN KEY (admin_user_id) REFERENCES cn_admin.admin_users (admin_user_id)
);

CREATE TABLE IF NOT EXISTS cn_admin.service_configs (
    service TEXT PRIMARY KEY,
    version BIGINT NOT NULL,
    config_json JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cn_admin.audit_logs (
    audit_id BIGSERIAL PRIMARY KEY,
    actor_admin_user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    target TEXT NOT NULL,
    diff_json JSONB NULL,
    request_id TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS audit_logs_action_idx
    ON cn_admin.audit_logs (action);

CREATE INDEX IF NOT EXISTS audit_logs_created_at_idx
    ON cn_admin.audit_logs (created_at);

CREATE TABLE IF NOT EXISTS cn_admin.service_health (
    service TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    checked_at TIMESTAMPTZ NOT NULL,
    details_json JSONB NULL
);

CREATE TABLE IF NOT EXISTS cn_user.subscriber_accounts (
    subscriber_pubkey TEXT PRIMARY KEY,
    status TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
