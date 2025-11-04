-- Direct messages table for NIP-04 encrypted conversations
CREATE TABLE IF NOT EXISTS direct_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_npub TEXT NOT NULL,
    conversation_npub TEXT NOT NULL,
    sender_npub TEXT NOT NULL,
    recipient_npub TEXT NOT NULL,
    event_id TEXT,
    client_message_id TEXT,
    payload_cipher_base64 TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    delivered INTEGER NOT NULL DEFAULT 0,
    direction TEXT NOT NULL CHECK(direction IN ('outbound', 'inbound')),
    UNIQUE(owner_npub, event_id),
    UNIQUE(owner_npub, client_message_id)
);

CREATE INDEX IF NOT EXISTS idx_direct_messages_owner_conversation_created
    ON direct_messages(owner_npub, conversation_npub, created_at DESC, event_id DESC);

CREATE INDEX IF NOT EXISTS idx_direct_messages_owner_delivered
    ON direct_messages(owner_npub, delivered);
