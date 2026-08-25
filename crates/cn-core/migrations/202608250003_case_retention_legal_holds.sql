-- #763: case retention, encrypted sensitive fields, and scoped legal holds.

ALTER TABLE cn_admin.reports
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ NOT NULL
    DEFAULT (NOW() + INTERVAL '180 days');
ALTER TABLE cn_admin.operator_actions
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ NOT NULL
    DEFAULT (NOW() + INTERVAL '365 days');
ALTER TABLE cn_legal.rights_requests
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ NOT NULL
    DEFAULT (NOW() + INTERVAL '730 days');
ALTER TABLE cn_legal.rights_request_events
    ADD COLUMN IF NOT EXISTS expires_at TIMESTAMPTZ NOT NULL
    DEFAULT (NOW() + INTERVAL '365 days');
ALTER TABLE cn_safety.signed_moderation_events
    ADD COLUMN IF NOT EXISTS retention_expires_at TIMESTAMPTZ NOT NULL
    DEFAULT (NOW() + INTERVAL '180 days');
ALTER TABLE cn_safety.risk_signals
    ADD COLUMN IF NOT EXISTS retention_expires_at TIMESTAMPTZ NOT NULL
    DEFAULT (NOW() + INTERVAL '180 days');

CREATE TABLE IF NOT EXISTS cn_legal.sensitive_items (
    id TEXT PRIMARY KEY,
    owner_kind TEXT NOT NULL CHECK (owner_kind IN ('report', 'rights_request')),
    owner_id TEXT NOT NULL CHECK (btrim(owner_id) <> ''),
    data_category TEXT NOT NULL CHECK (data_category IN (
        'report_contact', 'rights_request_contact',
        'rights_request_identity', 'rights_request_evidence'
    )),
    nonce BYTEA NOT NULL CHECK (octet_length(nonce) = 24),
    ciphertext BYTEA NOT NULL CHECK (octet_length(ciphertext) > 16),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    UNIQUE (owner_kind, owner_id, data_category)
);
CREATE INDEX IF NOT EXISTS idx_cn_legal_sensitive_items_expiry
    ON cn_legal.sensitive_items (expires_at);

CREATE TABLE IF NOT EXISTS cn_legal.legal_holds (
    id TEXT PRIMARY KEY,
    target_kind TEXT NOT NULL CHECK (target_kind IN ('report', 'rights_request')),
    target_id TEXT NOT NULL CHECK (btrim(target_id) <> ''),
    data_categories TEXT[] NOT NULL CHECK (cardinality(data_categories) > 0),
    basis TEXT NOT NULL CHECK (btrim(basis) <> ''),
    release_condition TEXT NOT NULL CHECK (btrim(release_condition) <> ''),
    started_by TEXT NOT NULL CHECK (btrim(started_by) <> ''),
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    released_by TEXT,
    released_at TIMESTAMPTZ,
    CHECK ((released_at IS NULL AND released_by IS NULL)
        OR (released_at IS NOT NULL AND btrim(released_by) <> ''))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_cn_legal_one_active_hold_per_target
    ON cn_legal.legal_holds (target_kind, target_id)
    WHERE released_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_cn_admin_reports_expiry ON cn_admin.reports (expires_at);
CREATE INDEX IF NOT EXISTS idx_cn_admin_operator_actions_expiry
    ON cn_admin.operator_actions (expires_at);
CREATE INDEX IF NOT EXISTS idx_cn_legal_rights_requests_expiry
    ON cn_legal.rights_requests (expires_at);
CREATE INDEX IF NOT EXISTS idx_cn_legal_rights_request_events_expiry
    ON cn_legal.rights_request_events (expires_at);

-- rights_request_events remain append-only for application operations. The retention
-- transaction explicitly opts into deletion after the configured expiry.
CREATE OR REPLACE FUNCTION cn_legal.reject_rights_request_event_mutation()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE'
       AND current_setting('kukuri.retention_cleanup', true) = 'on' THEN
        RETURN OLD;
    END IF;
    IF TG_OP = 'UPDATE'
       AND current_setting('kukuri.retention_reconcile', true) = 'on'
       AND (to_jsonb(NEW) - 'expires_at') = (to_jsonb(OLD) - 'expires_at') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'rights_request_events are append-only';
END;
$$;

CREATE OR REPLACE FUNCTION cn_admin.reject_operator_action_mutation()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE'
       AND current_setting('kukuri.retention_cleanup', true) = 'on' THEN
        RETURN OLD;
    END IF;
    IF TG_OP = 'UPDATE'
       AND current_setting('kukuri.retention_reconcile', true) = 'on'
       AND (to_jsonb(NEW) - 'expires_at') = (to_jsonb(OLD) - 'expires_at') THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'operator_actions are append-only';
END;
$$;
DROP TRIGGER IF EXISTS trg_cn_admin_operator_actions_append_only
    ON cn_admin.operator_actions;
CREATE TRIGGER trg_cn_admin_operator_actions_append_only
BEFORE UPDATE OR DELETE ON cn_admin.operator_actions
FOR EACH ROW EXECUTE FUNCTION cn_admin.reject_operator_action_mutation();
