ALTER TABLE object_index_cache ADD COLUMN content_labels_json TEXT NOT NULL DEFAULT '[]';

CREATE TABLE IF NOT EXISTS adult_media_hashes (
    blob_hash TEXT PRIMARY KEY NOT NULL,
    marked_at INTEGER NOT NULL DEFAULT 0
);
