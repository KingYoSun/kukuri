-- #760: accountless, node-local rights-infringement request intake.
CREATE SCHEMA IF NOT EXISTS cn_legal;

CREATE TABLE IF NOT EXISTS cn_legal.rights_requests (
    id TEXT PRIMARY KEY,
    tracking_secret_hash TEXT NOT NULL CHECK (length(tracking_secret_hash) = 64),
    scope_revision TEXT NOT NULL CHECK (btrim(scope_revision) <> ''),
    scope_status TEXT NOT NULL CHECK (scope_status IN (
        'verified_scope', 'unverified_scope', 'out_of_scope'
    )),
    status TEXT NOT NULL CHECK (status IN (
        'received', 'needs_information', 'reviewing', 'sender_contacting',
        'actioned', 'declined', 'out_of_scope', 'withdrawn'
    )),
    subject_kind TEXT NOT NULL,
    subject_id TEXT NOT NULL CHECK (btrim(subject_id) <> ''),
    requested_capabilities TEXT[] NOT NULL,
    -- Dedicated schema: PII and rights assertions never enter cn_admin.reports.
    request_data JSONB NOT NULL,
    version INTEGER NOT NULL DEFAULT 1 CHECK (version > 0),
    public_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cn_legal_rights_requests_created_at
    ON cn_legal.rights_requests (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_cn_legal_rights_requests_status
    ON cn_legal.rights_requests (status, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_cn_legal_rights_requests_subject
    ON cn_legal.rights_requests (subject_kind, subject_id);

CREATE TABLE IF NOT EXISTS cn_legal.rights_request_events (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES cn_legal.rights_requests (id),
    actor TEXT NOT NULL CHECK (btrim(actor) <> ''),
    action TEXT NOT NULL CHECK (btrim(action) <> ''),
    from_status TEXT,
    to_status TEXT NOT NULL,
    public_message TEXT,
    delivery_status TEXT NOT NULL DEFAULT 'status_surface',
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cn_legal_rights_request_events_request
    ON cn_legal.rights_request_events (request_id, occurred_at ASC);

CREATE OR REPLACE FUNCTION cn_legal.reject_rights_request_event_mutation()
RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'rights_request_events are append-only';
END;
$$;

DROP TRIGGER IF EXISTS trg_cn_legal_rights_request_events_append_only
    ON cn_legal.rights_request_events;
CREATE TRIGGER trg_cn_legal_rights_request_events_append_only
BEFORE UPDATE OR DELETE ON cn_legal.rights_request_events
FOR EACH ROW EXECUTE FUNCTION cn_legal.reject_rights_request_event_mutation();
