CREATE SCHEMA IF NOT EXISTS cn_metaverse;

CREATE TABLE cn_metaverse.dome_hosting_assignments (
    instance_id TEXT PRIMARY KEY,
    owner_pubkey TEXT NOT NULL,
    lease_id TEXT NOT NULL,
    lease_epoch BIGINT NOT NULL CHECK (lease_epoch > 0),
    expires_at BIGINT NOT NULL,
    session_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'active', 'closed')),
    signed_lease_json JSONB NOT NULL,
    instance_manifest_json JSONB NOT NULL,
    preset_manifest_json JSONB NOT NULL,
    signed_acceptance_json JSONB NOT NULL,
    signed_activation_json JSONB,
    signed_close_json JSONB,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_dome_hosting_active_lease
    ON cn_metaverse.dome_hosting_assignments (instance_id)
    WHERE status IN ('pending', 'active');

CREATE INDEX idx_dome_hosting_recovery
    ON cn_metaverse.dome_hosting_assignments (status, expires_at);
