ALTER TABLE profile_cache
    DROP COLUMN picture_bytes;

ALTER TABLE profile_cache
    DROP COLUMN picture_mime;

ALTER TABLE profile_cache
    DROP COLUMN picture_blob_hash;

ALTER TABLE profiles
    DROP COLUMN picture_bytes;

ALTER TABLE profiles
    DROP COLUMN picture_mime;

ALTER TABLE profiles
    DROP COLUMN picture_blob_hash;
