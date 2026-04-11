ALTER TABLE object_index_cache
    ADD COLUMN attachments_json TEXT NOT NULL DEFAULT '[]';
