CREATE TABLE dome_connection_projection_cache (
  context_id TEXT PRIMARY KEY,
  topic_id TEXT NOT NULL,
  channel_id TEXT NOT NULL DEFAULT '',
  snapshot_json TEXT NOT NULL,
  topology_digest TEXT NOT NULL,
  derived_at INTEGER NOT NULL,
  projection_version INTEGER NOT NULL
);

CREATE INDEX idx_dome_connection_projection_topic_channel
  ON dome_connection_projection_cache(topic_id, channel_id);
