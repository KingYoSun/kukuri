pub(super) const INSERT_POST_EVENT: &str = r#"
    INSERT INTO events (event_id, public_key, content, kind, tags, created_at)
    VALUES (?, ?, ?, ?, ?, ?)
"#;

pub(super) const SELECT_POST_BY_ID: &str = r#"
    SELECT event_id, public_key, content, created_at, tags
    FROM events
    WHERE event_id = ? AND kind = 1
"#;

pub(super) const SELECT_POSTS_BY_TOPIC: &str = r#"
    SELECT event_id, public_key, content, created_at, tags
    FROM events
    WHERE kind = 1
    AND tags LIKE '%' || ? || '%'
    ORDER BY created_at DESC
    LIMIT ?
"#;

pub(super) const UPDATE_POST_CONTENT: &str = r#"
    UPDATE events
    SET content = ?, updated_at = ?
    WHERE event_id = ?
"#;

pub(super) const MARK_POST_DELETED: &str = r#"
    UPDATE events
    SET deleted = 1, updated_at = ?
    WHERE event_id = ?
"#;

pub(super) const SELECT_UNSYNC_POSTS: &str = r#"
    SELECT event_id, public_key, content, created_at, tags
    FROM events
    WHERE kind = 1
    AND (sync_status IS NULL OR sync_status = 0)
    ORDER BY created_at DESC
"#;

pub(super) const MARK_POST_SYNCED: &str = r#"
    UPDATE events
    SET sync_status = 1, sync_event_id = ?, synced_at = ?
    WHERE event_id = ?
"#;

pub(super) const SELECT_POSTS_BY_AUTHOR: &str = r#"
    SELECT event_id, public_key, content, created_at, tags
    FROM events
    WHERE kind = 1 AND public_key = ?
    ORDER BY created_at DESC
    LIMIT ?
"#;

pub(super) const SELECT_RECENT_POSTS: &str = r#"
    SELECT event_id, public_key, content, created_at, tags
    FROM events
    WHERE kind = 1
    ORDER BY created_at DESC
    LIMIT ?
"#;

pub(super) const INSERT_TOPIC: &str = r#"
    INSERT INTO topics (topic_id, name, description, created_at, updated_at)
    VALUES (?, ?, ?, ?, ?)
"#;

pub(super) const SELECT_TOPIC_BY_ID: &str = r#"
    SELECT topic_id, name, description, created_at, updated_at, member_count, post_count
    FROM topics
    WHERE topic_id = ?
"#;

pub(super) const SELECT_ALL_TOPICS: &str = r#"
    SELECT topic_id, name, description, created_at, updated_at, member_count, post_count
    FROM topics
    ORDER BY created_at ASC
"#;

pub(super) const SELECT_JOINED_TOPICS: &str = r#"
    SELECT t.topic_id, t.name, t.description, t.created_at, t.updated_at, t.member_count, t.post_count
    FROM topics t
    INNER JOIN user_topics ut ON t.topic_id = ut.topic_id
    WHERE ut.is_joined = 1 AND ut.user_pubkey = ?
    ORDER BY t.created_at ASC
"#;

pub(super) const UPDATE_TOPIC: &str = r#"
    UPDATE topics
    SET name = ?, description = ?, updated_at = ?
    WHERE topic_id = ?
"#;

pub(super) const DELETE_USER_TOPICS_BY_TOPIC: &str = r#"
    DELETE FROM user_topics
    WHERE topic_id = ?
"#;

pub(super) const DELETE_TOPIC: &str = r#"
    DELETE FROM topics
    WHERE topic_id = ?
"#;

pub(super) const UPSERT_USER_TOPIC: &str = r#"
    INSERT INTO user_topics (topic_id, user_pubkey, is_joined, joined_at, left_at)
    VALUES (?1, ?2, 1, ?3, NULL)
    ON CONFLICT(topic_id, user_pubkey) DO UPDATE SET
        is_joined = 1,
        joined_at = excluded.joined_at,
        left_at = NULL
"#;

pub(super) const SELECT_TOPIC_MEMBER_COUNT: &str = r#"
    SELECT COUNT(*) as count
    FROM user_topics
    WHERE topic_id = ?1 AND is_joined = 1
"#;

pub(super) const UPDATE_TOPIC_MEMBER_COUNT: &str = r#"
    UPDATE topics
    SET member_count = ?1, updated_at = ?2
    WHERE topic_id = ?3
"#;

pub(super) const MARK_TOPIC_LEFT: &str = r#"
    UPDATE user_topics
    SET is_joined = 0, left_at = ?1
    WHERE topic_id = ?2 AND user_pubkey = ?3
"#;

pub(super) const UPDATE_TOPIC_STATS: &str = r#"
    UPDATE topics
    SET member_count = ?, post_count = ?, updated_at = ?
    WHERE topic_id = ?
"#;

pub(super) const INSERT_USER: &str = r#"
    INSERT INTO users (npub, pubkey, display_name, bio, avatar_url, created_at, updated_at)
    VALUES (?, ?, ?, ?, ?, ?, ?)
"#;

pub(super) const SELECT_USER_BY_NPUB: &str = r#"
    SELECT npub, pubkey, display_name, bio, avatar_url, created_at, updated_at
    FROM users
    WHERE npub = ?
"#;

pub(super) const SELECT_USER_BY_PUBKEY: &str = r#"
    SELECT npub, pubkey, display_name, bio, avatar_url, created_at, updated_at
    FROM users
    WHERE pubkey = ?
"#;

pub(super) const UPDATE_USER: &str = r#"
    UPDATE users
    SET display_name = ?, bio = ?, avatar_url = ?, updated_at = ?
    WHERE npub = ?
"#;

pub(super) const DELETE_USER: &str = r#"
    DELETE FROM users
    WHERE npub = ?
"#;

pub(super) const SELECT_FOLLOWERS: &str = r#"
    SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at
    FROM users u
    INNER JOIN follows f ON u.pubkey = f.follower_pubkey
    WHERE f.followed_pubkey = (SELECT pubkey FROM users WHERE npub = ?)
"#;

pub(super) const SELECT_FOLLOWING: &str = r#"
    SELECT u.npub, u.pubkey, u.display_name, u.bio, u.avatar_url, u.created_at, u.updated_at
    FROM users u
    INNER JOIN follows f ON u.pubkey = f.followed_pubkey
    WHERE f.follower_pubkey = (SELECT pubkey FROM users WHERE npub = ?)
"#;

pub(super) const INSERT_EVENT: &str = r#"
    INSERT INTO events (event_id, public_key, content, kind, tags, created_at, sig)
    VALUES (?, ?, ?, ?, ?, ?, ?)
"#;

pub(super) const SELECT_EVENT_BY_ID: &str = r#"
    SELECT event_id, public_key, content, kind, tags, created_at, sig
    FROM events
    WHERE event_id = ?
"#;

pub(super) const SELECT_EVENTS_BY_KIND: &str = r#"
    SELECT event_id, public_key, content, kind, tags, created_at, sig
    FROM events
    WHERE kind = ?
    ORDER BY created_at DESC
    LIMIT ?
"#;

pub(super) const SELECT_EVENTS_BY_AUTHOR: &str = r#"
    SELECT event_id, public_key, content, kind, tags, created_at, sig
    FROM events
    WHERE public_key = ?
    ORDER BY created_at DESC
    LIMIT ?
"#;

pub(super) const MARK_EVENT_DELETED: &str = r#"
    UPDATE events
    SET deleted = 1, updated_at = ?
    WHERE event_id = ?
"#;

pub(super) const SELECT_UNSYNC_EVENTS: &str = r#"
    SELECT event_id, public_key, content, kind, tags, created_at, sig
    FROM events
    WHERE sync_status IS NULL OR sync_status = 0
    ORDER BY created_at DESC
"#;

pub(super) const MARK_EVENT_SYNCED: &str = r#"
    UPDATE events
    SET sync_status = 1, synced_at = ?
    WHERE event_id = ?
"#;

pub(super) const INSERT_EVENT_TOPIC: &str = r#"
    INSERT OR IGNORE INTO event_topics (event_id, topic_id, created_at)
    VALUES (?1, ?2, ?3)
"#;

pub(super) const SELECT_EVENT_TOPICS: &str = r#"
    SELECT topic_id FROM event_topics WHERE event_id = ?1
"#;
