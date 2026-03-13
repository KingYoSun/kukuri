CREATE TABLE IF NOT EXISTS direct_message_conversations (
    owner_npub TEXT NOT NULL,
    conversation_npub TEXT NOT NULL,
    last_message_id INTEGER,
    last_message_created_at INTEGER,
    last_read_at INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (owner_npub, conversation_npub)
);

CREATE INDEX IF NOT EXISTS idx_dm_conversations_owner_ordered
    ON direct_message_conversations(owner_npub, last_message_created_at DESC, conversation_npub ASC);
