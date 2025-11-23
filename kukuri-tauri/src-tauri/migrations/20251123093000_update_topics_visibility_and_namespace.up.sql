ALTER TABLE topics ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public';

UPDATE topics
SET topic_id = 'kukuri:tauri:public',
    name = '#public'
WHERE topic_id = 'public';

UPDATE user_topics
SET topic_id = 'kukuri:tauri:public'
WHERE topic_id = 'public';

INSERT OR IGNORE INTO topics (
    topic_id,
    name,
    description,
    created_at,
    updated_at,
    member_count,
    post_count,
    visibility
) VALUES (
    'kukuri:tauri:public',
    '#public',
    '公開タイムライン',
    unixepoch() * 1000,
    unixepoch() * 1000,
    0,
    0,
    'public'
);
