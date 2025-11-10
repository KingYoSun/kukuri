ALTER TABLE cache_metadata ADD COLUMN doc_version INTEGER;
ALTER TABLE cache_metadata ADD COLUMN blob_hash TEXT;
ALTER TABLE cache_metadata ADD COLUMN payload_bytes INTEGER;

CREATE INDEX IF NOT EXISTS idx_cache_metadata_doc_version ON cache_metadata(doc_version);
CREATE INDEX IF NOT EXISTS idx_cache_metadata_blob_hash ON cache_metadata(blob_hash);
