DROP INDEX IF EXISTS idx_live_presence_cache_identity;

DROP INDEX IF EXISTS idx_live_presence_cache_expiry;
CREATE INDEX IF NOT EXISTS idx_live_presence_cache_expiry
    ON live_presence_cache(expires_at ASC);

DROP INDEX IF EXISTS idx_game_room_cache_topic_updated;
CREATE INDEX IF NOT EXISTS idx_game_room_cache_topic_updated
    ON game_room_cache(topic_id, updated_at DESC, room_id DESC);

DROP INDEX IF EXISTS idx_live_session_cache_topic_started;
CREATE INDEX IF NOT EXISTS idx_live_session_cache_topic_started
    ON live_session_cache(topic_id, started_at DESC, session_id DESC);

DROP INDEX IF EXISTS idx_object_thread_cache_topic_root_created;
CREATE INDEX IF NOT EXISTS idx_object_thread_cache_topic_root_created
    ON object_thread_cache(topic_id, root_object_id, created_at ASC, object_id ASC);

DROP INDEX IF EXISTS idx_object_index_cache_topic_created;
CREATE INDEX IF NOT EXISTS idx_object_index_cache_topic_created
    ON object_index_cache(topic_id, created_at DESC, object_id DESC);
