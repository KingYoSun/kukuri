-- イベントとトピックのマッピングテーブル
CREATE TABLE IF NOT EXISTS event_topics (
    event_id TEXT NOT NULL,
    topic_id TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (event_id, topic_id),
    FOREIGN KEY (event_id) REFERENCES events(event_id)
);

-- 参照高速化のためのインデックス
CREATE INDEX IF NOT EXISTS idx_event_topics_event_id ON event_topics (event_id);
CREATE INDEX IF NOT EXISTS idx_event_topics_topic_id ON event_topics (topic_id);

