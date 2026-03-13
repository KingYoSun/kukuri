-- ロールバック用：すべてのテーブルを削除
DROP TABLE IF EXISTS sync_status;
DROP TABLE IF EXISTS sync_queue;
DROP TABLE IF EXISTS optimistic_updates;
DROP TABLE IF EXISTS cache_metadata;
DROP TABLE IF EXISTS offline_actions;
DROP TABLE IF EXISTS bookmarks;
DROP TABLE IF EXISTS reactions;
DROP TABLE IF EXISTS follows;
DROP TABLE IF EXISTS user_topics;
DROP TABLE IF EXISTS profiles;
DROP TABLE IF EXISTS relays;
DROP TABLE IF EXISTS topics;
DROP TABLE IF EXISTS events;
DROP TABLE IF EXISTS users;
