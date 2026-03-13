BEGIN TRANSACTION;

CREATE TABLE topics_backup AS
SELECT
    CASE
        WHEN topic_id IN ('kukuri:tauri:public', 'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0') THEN 'public'
        ELSE topic_id
    END AS topic_id,
    name,
    description,
    created_at,
    updated_at,
    member_count,
    post_count
FROM topics;

DROP TABLE topics;

CREATE TABLE topics (
    topic_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    member_count INTEGER DEFAULT 0,
    post_count INTEGER DEFAULT 0
);

INSERT INTO topics (
    topic_id,
    name,
    description,
    created_at,
    updated_at,
    member_count,
    post_count
)
SELECT
    topic_id,
    name,
    description,
    created_at,
    updated_at,
    member_count,
    post_count
FROM topics_backup;

DROP TABLE topics_backup;

UPDATE user_topics
SET topic_id = 'public'
WHERE topic_id IN ('kukuri:tauri:public', 'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0');

COMMIT;
