DROP INDEX IF EXISTS idx_dm_tombstones_dm_id;
DROP TABLE IF EXISTS dm_message_tombstones;

DROP INDEX IF EXISTS idx_dm_outbox_created_at;
DROP TABLE IF EXISTS dm_outbox;

DROP INDEX IF EXISTS idx_dm_messages_timeline;
DROP TABLE IF EXISTS dm_messages;

DROP INDEX IF EXISTS idx_dm_conversations_updated_at;
DROP TABLE IF EXISTS dm_conversations;
