ALTER TABLE profiles
    ADD COLUMN picture_blob_hash TEXT;

ALTER TABLE profiles
    ADD COLUMN picture_mime TEXT;

ALTER TABLE profiles
    ADD COLUMN picture_bytes INTEGER;

ALTER TABLE profile_cache
    ADD COLUMN picture_blob_hash TEXT;

ALTER TABLE profile_cache
    ADD COLUMN picture_mime TEXT;

ALTER TABLE profile_cache
    ADD COLUMN picture_bytes INTEGER;
