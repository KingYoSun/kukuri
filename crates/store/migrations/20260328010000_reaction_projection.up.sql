CREATE TABLE IF NOT EXISTS reaction_cache (
    source_replica_id TEXT NOT NULL,
    target_object_id TEXT NOT NULL,
    reaction_id TEXT NOT NULL,
    author_pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    reaction_key_kind TEXT NOT NULL,
    normalized_reaction_key TEXT NOT NULL,
    emoji TEXT,
    custom_asset_id TEXT,
    custom_asset_snapshot_json TEXT,
    status TEXT NOT NULL,
    source_key TEXT NOT NULL,
    source_envelope_id TEXT NOT NULL,
    derived_at INTEGER NOT NULL,
    projection_version INTEGER NOT NULL,
    PRIMARY KEY (source_replica_id, target_object_id, reaction_id)
);

CREATE INDEX IF NOT EXISTS idx_reaction_cache_target
    ON reaction_cache(source_replica_id, target_object_id, normalized_reaction_key, reaction_id);

CREATE TABLE IF NOT EXISTS bookmarked_custom_reactions (
    asset_id TEXT PRIMARY KEY,
    owner_pubkey TEXT NOT NULL,
    blob_hash TEXT NOT NULL,
    mime TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    bookmarked_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bookmarked_custom_reactions_bookmarked_at
    ON bookmarked_custom_reactions(bookmarked_at DESC, asset_id DESC);
