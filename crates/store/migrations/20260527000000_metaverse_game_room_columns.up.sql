ALTER TABLE game_room_cache
  ADD COLUMN room_kind TEXT NOT NULL DEFAULT 'score_game';

ALTER TABLE game_room_cache
  ADD COLUMN metaverse_json TEXT;

CREATE INDEX IF NOT EXISTS idx_game_room_cache_topic_kind_updated
  ON game_room_cache(topic_id, room_kind, updated_at DESC, room_id DESC);
