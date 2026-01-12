ALTER TABLE topics ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public';

UPDATE topics
SET topic_id = 'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0',
    name = '#public'
WHERE topic_id IN ('public', 'kukuri:tauri:public');

UPDATE user_topics
SET topic_id = 'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0'
WHERE topic_id IN ('public', 'kukuri:tauri:public');

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
    'kukuri:tauri:731051a1c14a65ee3735ee4ab3b97198cae1633700f9b87fcde205e64c5a56b0',
    '#public',
    '公開タイムライン',
    unixepoch() * 1000,
    unixepoch() * 1000,
    0,
    0,
    'public'
);
