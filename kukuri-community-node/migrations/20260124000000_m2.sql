CREATE SCHEMA IF NOT EXISTS cn_bootstrap;
CREATE SCHEMA IF NOT EXISTS cn_relay;

CREATE TABLE IF NOT EXISTS cn_admin.topic_services (
    topic_id TEXT NOT NULL,
    role TEXT NOT NULL,
    scope TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT NOT NULL,
    PRIMARY KEY (topic_id, role, scope)
);

CREATE TABLE IF NOT EXISTS cn_admin.node_subscriptions (
    topic_id TEXT PRIMARY KEY,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ref_count BIGINT NOT NULL DEFAULT 0,
    ingest_policy JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_admin.policies (
    policy_id TEXT PRIMARY KEY,
    type TEXT NOT NULL,
    version TEXT NOT NULL,
    locale TEXT NOT NULL,
    title TEXT NOT NULL,
    content_md TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    published_at TIMESTAMPTZ NULL,
    effective_at TIMESTAMPTZ NULL,
    is_current BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS policies_type_locale_idx
    ON cn_admin.policies (type, locale);

CREATE TABLE IF NOT EXISTS cn_admin.topic_scope_state (
    topic_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    current_epoch INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (topic_id, scope)
);

CREATE TABLE IF NOT EXISTS cn_admin.topic_scope_keys (
    topic_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    epoch INT NOT NULL,
    key_ciphertext TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (topic_id, scope, epoch)
);

CREATE TABLE IF NOT EXISTS cn_bootstrap.events (
    event_id TEXT PRIMARY KEY,
    kind INT NOT NULL,
    d_tag TEXT NOT NULL,
    topic_id TEXT NULL,
    role TEXT NULL,
    scope TEXT NULL,
    event_json JSONB NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX IF NOT EXISTS bootstrap_events_kind_idx
    ON cn_bootstrap.events (kind);

CREATE INDEX IF NOT EXISTS bootstrap_events_topic_idx
    ON cn_bootstrap.events (topic_id);

CREATE TABLE IF NOT EXISTS cn_relay.event_dedupe (
    event_id TEXT PRIMARY KEY,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    seen_count BIGINT NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS cn_relay.events (
    event_id TEXT PRIMARY KEY,
    pubkey TEXT NOT NULL,
    kind INT NOT NULL,
    created_at BIGINT NOT NULL,
    tags JSONB NOT NULL,
    content TEXT NOT NULL,
    sig TEXT NOT NULL,
    raw_json JSONB NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMPTZ NULL,
    is_ephemeral BOOLEAN NOT NULL DEFAULT FALSE,
    is_current BOOLEAN NOT NULL DEFAULT TRUE,
    replaceable_key TEXT NULL,
    addressable_key TEXT NULL,
    expires_at BIGINT NULL
);

CREATE INDEX IF NOT EXISTS relay_events_kind_idx
    ON cn_relay.events (kind);

CREATE INDEX IF NOT EXISTS relay_events_created_at_idx
    ON cn_relay.events (created_at);

CREATE INDEX IF NOT EXISTS relay_events_ingested_at_idx
    ON cn_relay.events (ingested_at);

CREATE TABLE IF NOT EXISTS cn_relay.event_topics (
    event_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    PRIMARY KEY (event_id, topic_id)
);

CREATE INDEX IF NOT EXISTS relay_event_topics_topic_idx
    ON cn_relay.event_topics (topic_id);

CREATE TABLE IF NOT EXISTS cn_relay.deletion_tombstones (
    tombstone_id BIGSERIAL PRIMARY KEY,
    target_event_id TEXT NULL,
    target_a TEXT NULL,
    deletion_event_id TEXT NOT NULL,
    requested_at BIGINT NOT NULL,
    applied_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS relay_tombstones_target_event_idx
    ON cn_relay.deletion_tombstones (target_event_id);

CREATE TABLE IF NOT EXISTS cn_relay.replaceable_current (
    replaceable_key TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    pubkey TEXT NOT NULL,
    kind INT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_relay.addressable_current (
    addressable_key TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    pubkey TEXT NOT NULL,
    kind INT NOT NULL,
    d_tag TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_relay.events_outbox (
    seq BIGSERIAL PRIMARY KEY,
    op TEXT NOT NULL,
    event_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    kind INT NOT NULL,
    created_at BIGINT NOT NULL,
    ingested_at TIMESTAMPTZ NOT NULL,
    effective_key TEXT NULL,
    reason TEXT NULL
);

CREATE INDEX IF NOT EXISTS relay_outbox_event_idx
    ON cn_relay.events_outbox (event_id);

CREATE TABLE IF NOT EXISTS cn_relay.consumer_offsets (
    consumer TEXT PRIMARY KEY,
    last_seq BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_user.auth_challenges (
    challenge TEXT PRIMARY KEY,
    pubkey TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_user.policy_consents (
    consent_id TEXT PRIMARY KEY,
    policy_id TEXT NOT NULL,
    accepter_pubkey TEXT NOT NULL,
    accepter_hmac TEXT NULL,
    accepted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip TEXT NULL,
    user_agent TEXT NULL
);

CREATE INDEX IF NOT EXISTS policy_consents_pubkey_idx
    ON cn_user.policy_consents (accepter_pubkey);

CREATE TABLE IF NOT EXISTS cn_user.topic_subscription_requests (
    request_id TEXT PRIMARY KEY,
    requester_pubkey TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    requested_services JSONB NOT NULL,
    status TEXT NOT NULL,
    review_note TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS topic_subscription_requests_pubkey_idx
    ON cn_user.topic_subscription_requests (requester_pubkey);

CREATE TABLE IF NOT EXISTS cn_user.topic_subscriptions (
    topic_id TEXT NOT NULL,
    subscriber_pubkey TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ NULL,
    PRIMARY KEY (topic_id, subscriber_pubkey)
);

CREATE TABLE IF NOT EXISTS cn_user.plans (
    plan_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS cn_user.plan_limits (
    plan_id TEXT NOT NULL,
    metric TEXT NOT NULL,
    "window" TEXT NOT NULL,
    "limit" BIGINT NOT NULL,
    PRIMARY KEY (plan_id, metric, "window")
);

CREATE TABLE IF NOT EXISTS cn_user.subscriptions (
    subscription_id TEXT PRIMARY KEY,
    subscriber_pubkey TEXT NOT NULL,
    plan_id TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS subscriptions_pubkey_idx
    ON cn_user.subscriptions (subscriber_pubkey);

CREATE TABLE IF NOT EXISTS cn_user.usage_counters_daily (
    subscriber_pubkey TEXT NOT NULL,
    metric TEXT NOT NULL,
    day DATE NOT NULL,
    count BIGINT NOT NULL,
    PRIMARY KEY (subscriber_pubkey, metric, day)
);

CREATE TABLE IF NOT EXISTS cn_user.usage_events (
    event_id BIGSERIAL PRIMARY KEY,
    subscriber_pubkey TEXT NOT NULL,
    metric TEXT NOT NULL,
    day DATE NOT NULL,
    request_id TEXT NULL,
    units BIGINT NOT NULL,
    outcome TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS usage_events_idempotency_idx
    ON cn_user.usage_events (subscriber_pubkey, metric, request_id)
    WHERE request_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS cn_user.invite_capabilities (
    topic_id TEXT NOT NULL,
    issuer_pubkey TEXT NOT NULL,
    nonce TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    max_uses INT NOT NULL,
    used_count INT NOT NULL DEFAULT 0,
    revoked_at TIMESTAMPTZ NULL,
    capability_event_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (nonce)
);

CREATE TABLE IF NOT EXISTS cn_user.topic_memberships (
    topic_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    pubkey TEXT NOT NULL,
    status TEXT NOT NULL,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ NULL,
    revoked_reason TEXT NULL,
    PRIMARY KEY (topic_id, scope, pubkey)
);

CREATE TABLE IF NOT EXISTS cn_user.key_envelopes (
    topic_id TEXT NOT NULL,
    scope TEXT NOT NULL,
    epoch INT NOT NULL,
    recipient_pubkey TEXT NOT NULL,
    key_envelope_event_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (topic_id, scope, epoch, recipient_pubkey)
);

CREATE TABLE IF NOT EXISTS cn_user.personal_data_export_requests (
    export_request_id TEXT PRIMARY KEY,
    requester_pubkey TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NULL,
    download_token TEXT NULL,
    download_expires_at TIMESTAMPTZ NULL,
    file_path TEXT NULL,
    error_message TEXT NULL
);

CREATE TABLE IF NOT EXISTS cn_user.personal_data_deletion_requests (
    deletion_request_id TEXT PRIMARY KEY,
    requester_pubkey TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NULL,
    error_message TEXT NULL
);

CREATE TABLE IF NOT EXISTS cn_user.reports (
    report_id TEXT PRIMARY KEY,
    reporter_pubkey TEXT NOT NULL,
    target TEXT NOT NULL,
    reason TEXT NOT NULL,
    report_event_json JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
