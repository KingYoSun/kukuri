-- #761: node-local legal transmission-prevention decisions.
CREATE SCHEMA IF NOT EXISTS cn_legal;

CREATE TABLE IF NOT EXISTS cn_legal.transmission_preventions (
    id TEXT PRIMARY KEY,
    subject_kind TEXT NOT NULL CHECK (subject_kind IN ('post', 'blob')),
    subject_id TEXT NOT NULL CHECK (btrim(subject_id) <> ''),
    basis_category TEXT NOT NULL CHECK (basis_category IN (
        'copyright', 'privacy', 'personality_rights', 'trademark', 'other_rights'
    )),
    capabilities TEXT[] NOT NULL CHECK (cardinality(capabilities) > 0),
    decided_by TEXT NOT NULL CHECK (btrim(decided_by) <> ''),
    decided_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    related_report_id TEXT,
    released_at TIMESTAMPTZ,
    released_by TEXT,
    release_reason TEXT,
    CHECK ((released_at IS NULL AND released_by IS NULL) OR
           (released_at IS NOT NULL AND btrim(released_by) <> ''))
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_cn_legal_active_transmission_prevention
    ON cn_legal.transmission_preventions (subject_kind, subject_id)
    WHERE released_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_cn_legal_transmission_prevention_subject
    ON cn_legal.transmission_preventions (subject_kind, subject_id, decided_at DESC);
