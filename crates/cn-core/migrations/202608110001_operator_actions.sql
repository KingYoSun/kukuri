-- #382: IAP 内部 admin surface から行う runtime 操作の append-only audit。
--
-- before / after には秘密・credential・report details・reporter contact を入れない。
CREATE TABLE IF NOT EXISTS cn_admin.operator_actions (
    id TEXT PRIMARY KEY,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    actor TEXT NOT NULL CHECK (btrim(actor) <> ''),
    action TEXT NOT NULL,
    target_kind TEXT NOT NULL,
    target_id TEXT NOT NULL,
    before_json JSONB NOT NULL,
    after_json JSONB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cn_admin_operator_actions_occurred_at
    ON cn_admin.operator_actions (occurred_at DESC);

CREATE OR REPLACE FUNCTION cn_admin.reject_operator_actions_mutation()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'cn_admin.operator_actions is append-only';
END;
$$;

DROP TRIGGER IF EXISTS trg_cn_admin_operator_actions_append_only
    ON cn_admin.operator_actions;
CREATE TRIGGER trg_cn_admin_operator_actions_append_only
BEFORE UPDATE OR DELETE ON cn_admin.operator_actions
FOR EACH ROW EXECUTE FUNCTION cn_admin.reject_operator_actions_mutation();
