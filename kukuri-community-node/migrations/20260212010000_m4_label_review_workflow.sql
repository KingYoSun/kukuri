ALTER TABLE cn_moderation.labels
    ADD COLUMN IF NOT EXISTS review_status TEXT NOT NULL DEFAULT 'active',
    ADD COLUMN IF NOT EXISTS review_reason TEXT NULL,
    ADD COLUMN IF NOT EXISTS reviewed_by TEXT NULL,
    ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
          FROM pg_constraint
         WHERE conname = 'moderation_labels_review_status_check'
           AND conrelid = 'cn_moderation.labels'::regclass
    ) THEN
        ALTER TABLE cn_moderation.labels
            ADD CONSTRAINT moderation_labels_review_status_check
            CHECK (review_status IN ('active', 'disabled'));
    END IF;
END;
$$;

DROP INDEX IF EXISTS cn_moderation.moderation_labels_rule_event_idx;

CREATE UNIQUE INDEX IF NOT EXISTS moderation_labels_rule_event_active_idx
    ON cn_moderation.labels (source_event_id, rule_id)
    WHERE source_event_id IS NOT NULL
      AND rule_id IS NOT NULL
      AND review_status = 'active';

CREATE INDEX IF NOT EXISTS moderation_labels_review_status_idx
    ON cn_moderation.labels (review_status, issued_at DESC);
