ALTER TABLE notifications ADD COLUMN content_labels_json TEXT;

UPDATE notifications
SET content_labels_json = (
    SELECT object_index_cache.content_labels_json
    FROM object_index_cache
    WHERE object_index_cache.object_id = notifications.object_id
)
WHERE notifications.object_id IS NOT NULL;
