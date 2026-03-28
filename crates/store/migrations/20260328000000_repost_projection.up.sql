ALTER TABLE object_index_cache
    ADD COLUMN object_kind TEXT NOT NULL DEFAULT 'post';

UPDATE object_index_cache
SET object_kind = CASE
    WHEN reply_to_object_id IS NOT NULL THEN 'comment'
    ELSE 'post'
END;

ALTER TABLE object_index_cache
    ADD COLUMN repost_of_json TEXT;
