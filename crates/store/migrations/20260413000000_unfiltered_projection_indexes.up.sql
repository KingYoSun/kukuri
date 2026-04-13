CREATE INDEX IF NOT EXISTS idx_object_index_cache_topic_created_all
    ON object_index_cache(topic_id, created_at DESC, object_id DESC);

CREATE INDEX IF NOT EXISTS idx_object_thread_cache_topic_root_created_all
    ON object_thread_cache(topic_id, root_object_id, created_at ASC, object_id ASC);

CREATE INDEX IF NOT EXISTS idx_live_session_cache_topic_started_all
    ON live_session_cache(topic_id, started_at DESC, session_id DESC);

CREATE INDEX IF NOT EXISTS idx_game_room_cache_topic_updated_all
    ON game_room_cache(topic_id, updated_at DESC, room_id DESC);
