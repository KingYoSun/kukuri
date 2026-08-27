CREATE TABLE dome_hosting_projection_cache (
  instance_id TEXT PRIMARY KEY,
  context_id TEXT NOT NULL,
  topic_id TEXT NOT NULL,
  channel_id TEXT NOT NULL DEFAULT '',
  state_json TEXT NOT NULL,
  lease_epoch INTEGER,
  session_id TEXT,
  derived_at INTEGER NOT NULL,
  projection_version INTEGER NOT NULL
);

CREATE INDEX idx_dome_hosting_projection_context
  ON dome_hosting_projection_cache(context_id);

CREATE INDEX idx_dome_hosting_projection_topic_channel
  ON dome_hosting_projection_cache(topic_id, channel_id);
