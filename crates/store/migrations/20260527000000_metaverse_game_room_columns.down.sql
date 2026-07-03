DROP INDEX IF EXISTS idx_game_room_cache_topic_kind_updated;

ALTER TABLE game_room_cache
  DROP COLUMN metaverse_json;

ALTER TABLE game_room_cache
  DROP COLUMN room_kind;
