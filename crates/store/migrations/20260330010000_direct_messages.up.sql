CREATE TABLE IF NOT EXISTS dm_conversations (
    dm_id TEXT PRIMARY KEY,
    peer_pubkey TEXT NOT NULL UNIQUE,
    updated_at INTEGER NOT NULL,
    last_message_at INTEGER,
    last_message_id TEXT,
    last_message_preview TEXT
);

CREATE INDEX IF NOT EXISTS idx_dm_conversations_updated_at
    ON dm_conversations(updated_at DESC, dm_id DESC);

CREATE TABLE IF NOT EXISTS dm_messages (
    dm_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    sender_pubkey TEXT NOT NULL,
    recipient_pubkey TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    text TEXT,
    reply_to_message_id TEXT,
    attachment_manifest_json TEXT,
    outgoing INTEGER NOT NULL DEFAULT 0,
    acked_at INTEGER,
    PRIMARY KEY (dm_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_messages_timeline
    ON dm_messages(dm_id, created_at DESC, message_id DESC);

CREATE TABLE IF NOT EXISTS dm_outbox (
    dm_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    peer_pubkey TEXT NOT NULL,
    frame_blob_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_attempt_at INTEGER,
    PRIMARY KEY (dm_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_outbox_created_at
    ON dm_outbox(created_at ASC, message_id ASC);

CREATE TABLE IF NOT EXISTS dm_message_tombstones (
    dm_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    deleted_at INTEGER NOT NULL,
    PRIMARY KEY (dm_id, message_id)
);

CREATE INDEX IF NOT EXISTS idx_dm_tombstones_dm_id
    ON dm_message_tombstones(dm_id, deleted_at DESC, message_id DESC);
