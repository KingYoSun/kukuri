pub(super) const INSERT_BOOKMARK: &str = r#"
    INSERT INTO bookmarks (id, user_pubkey, post_id, created_at)
    VALUES (?1, ?2, ?3, ?4)
    ON CONFLICT(user_pubkey, post_id) DO NOTHING
"#;

pub(super) const DELETE_BOOKMARK: &str = r#"
    DELETE FROM bookmarks
    WHERE user_pubkey = ?1 AND post_id = ?2
"#;

pub(super) const SELECT_BOOKMARK_BY_USER_AND_POST: &str = r#"
    SELECT id, user_pubkey, post_id, created_at
    FROM bookmarks
    WHERE user_pubkey = ?1 AND post_id = ?2
"#;

pub(super) const SELECT_BOOKMARKS_BY_USER: &str = r#"
    SELECT id, user_pubkey, post_id, created_at
    FROM bookmarks
    WHERE user_pubkey = ?
    ORDER BY created_at DESC
"#;

pub(super) const INSERT_DIRECT_MESSAGE: &str = r#"
    INSERT INTO direct_messages (
        owner_npub,
        conversation_npub,
        sender_npub,
        recipient_npub,
        event_id,
        client_message_id,
        payload_cipher_base64,
        created_at,
        delivered,
        direction
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
"#;

pub(super) const MARK_DIRECT_MESSAGE_DELIVERED_BY_CLIENT_ID: &str = r#"
    UPDATE direct_messages
    SET delivered = ?4,
        event_id = COALESCE(?3, event_id)
    WHERE owner_npub = ?1
      AND client_message_id = ?2
"#;

pub(super) const SELECT_DIRECT_MESSAGE_BY_ID: &str = r#"
    SELECT id,
           owner_npub,
           conversation_npub,
           sender_npub,
           recipient_npub,
           event_id,
           client_message_id,
           payload_cipher_base64,
           created_at,
           delivered,
           direction
    FROM direct_messages
    WHERE id = ?1
"#;

pub(super) const UPDATE_DM_CONVERSATION_LAST_MESSAGE: &str = r#"
    UPDATE direct_message_conversations
    SET last_message_id = ?3,
        last_message_created_at = ?4
    WHERE owner_npub = ?1 AND conversation_npub = ?2
"#;

pub(super) const INSERT_DM_CONVERSATION: &str = r#"
    INSERT INTO direct_message_conversations (
        owner_npub,
        conversation_npub,
        last_message_id,
        last_message_created_at,
        last_read_at
    ) VALUES (?1, ?2, ?3, ?4, ?5)
"#;

pub(super) const MARK_DM_CONVERSATION_READ: &str = r#"
    UPDATE direct_message_conversations
    SET last_read_at = MAX(last_read_at, ?3)
    WHERE owner_npub = ?1 AND conversation_npub = ?2
"#;

pub(super) const INSERT_POST_EVENT: &str = r#"
    INSERT INTO events (event_id, public_key, content, kind, tags, created_at)
    VALUES (?, ?, ?, ?, ?, ?)
"#;

pub(super) const UPSERT_EVENT_THREAD: &str = r#"
    INSERT INTO event_threads (
        event_id,
        topic_id,
        thread_namespace,
        thread_uuid,
        root_event_id,
        parent_event_id,
        created_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
    ON CONFLICT(event_id) DO UPDATE SET
        topic_id = excluded.topic_id,
        thread_namespace = excluded.thread_namespace,
        thread_uuid = excluded.thread_uuid,
        root_event_id = excluded.root_event_id,
        parent_event_id = excluded.parent_event_id
"#;

pub(super) const SELECT_EVENT_THREAD_BY_EVENT: &str = r#"
    SELECT
        event_id,
        topic_id,
        thread_namespace,
        thread_uuid,
        root_event_id,
        parent_event_id
    FROM event_threads
    WHERE topic_id = ?1
      AND event_id = ?2
    LIMIT 1
"#;

pub(super) const SELECT_SYNC_EVENT_ID_BY_EVENT: &str = r#"
    SELECT sync_event_id
    FROM events
    WHERE event_id = ?1
      AND kind = 1
    LIMIT 1
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

pub(super) const SELECT_TOPIC_TIMELINE_SUMMARIES: &str = r#"
    WITH threaded AS (
        SELECT
            et.thread_uuid AS thread_uuid,
            et.root_event_id AS root_event_id,
            (
                SELECT et2.event_id
                FROM event_threads et2
                INNER JOIN events e2 ON e2.event_id = et2.event_id
                WHERE et2.topic_id = et.topic_id
                  AND et2.root_event_id = et.root_event_id
                  AND et2.parent_event_id IS NOT NULL
                  AND e2.kind = 1
                  AND e2.deleted = 0
                ORDER BY e2.created_at ASC
                LIMIT 1
            ) AS first_reply_event_id,
            SUM(CASE WHEN et.parent_event_id IS NOT NULL THEN 1 ELSE 0 END) AS reply_count,
            MAX(e.created_at) AS last_activity_at
        FROM event_threads et
        INNER JOIN events e ON e.event_id = et.event_id
        WHERE et.topic_id = ?1
          AND e.kind = 1
          AND e.deleted = 0
          AND EXISTS (
              SELECT 1
              FROM events er
              WHERE er.event_id = et.root_event_id
                AND er.kind = 1
                AND er.deleted = 0
          )
        GROUP BY et.thread_uuid, et.root_event_id
    )
    SELECT
        thread_uuid,
        root_event_id,
        first_reply_event_id,
        reply_count,
        last_activity_at
    FROM threaded
    ORDER BY last_activity_at DESC, root_event_id DESC
    LIMIT ?2
"#;

pub(super) const SELECT_POSTS_BY_THREAD: &str = r#"
    SELECT
        e.event_id,
        e.public_key,
        e.content,
        e.created_at,
        e.tags,
        et.thread_namespace AS thread_namespace,
        et.thread_uuid AS thread_uuid,
        et.root_event_id AS thread_root_event_id,
        et.parent_event_id AS thread_parent_event_id
    FROM event_threads et
    INNER JOIN events e ON e.event_id = et.event_id
    WHERE et.topic_id = ?1
      AND et.thread_uuid = ?2
      AND e.kind = 1
      AND e.deleted = 0
    ORDER BY e.created_at ASC
    LIMIT ?3
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
    INSERT INTO topics (topic_id, name, description, created_at, updated_at, visibility)
    VALUES (?, ?, ?, ?, ?, ?)
"#;

pub(super) const INSERT_PENDING_TOPIC: &str = r#"
    INSERT INTO topics_pending (
        pending_id,
        user_pubkey,
        name,
        description,
        status,
        offline_action_id,
        synced_topic_id,
        error_message,
        created_at,
        updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
"#;

pub(super) const SELECT_TOPIC_BY_ID: &str = r#"
    SELECT topic_id, name, description, created_at, updated_at, member_count, post_count, visibility
    FROM topics
    WHERE topic_id = ?
"#;

pub(super) const SELECT_ALL_TOPICS: &str = r#"
    SELECT topic_id, name, description, created_at, updated_at, member_count, post_count, visibility
    FROM topics
    ORDER BY created_at ASC
"#;

pub(super) const SELECT_PENDING_TOPIC_BY_ID: &str = r#"
    SELECT pending_id,
           user_pubkey,
           name,
           description,
           status,
           offline_action_id,
           synced_topic_id,
           error_message,
           created_at,
           updated_at
    FROM topics_pending
    WHERE pending_id = ?1
"#;

pub(super) const SELECT_PENDING_TOPICS_BY_USER: &str = r#"
    SELECT pending_id,
           user_pubkey,
           name,
           description,
           status,
           offline_action_id,
           synced_topic_id,
           error_message,
           created_at,
           updated_at
    FROM topics_pending
    WHERE user_pubkey = ?1
    ORDER BY created_at DESC
"#;

pub(super) const UPDATE_PENDING_TOPIC_STATUS: &str = r#"
    UPDATE topics_pending
    SET status = ?2,
        synced_topic_id = ?3,
        error_message = ?4,
        updated_at = ?5
    WHERE pending_id = ?1
"#;

pub(super) const DELETE_PENDING_TOPIC: &str = r#"
    DELETE FROM topics_pending
    WHERE pending_id = ?1
"#;

pub(super) const SELECT_JOINED_TOPICS: &str = r#"
    SELECT t.topic_id, t.name, t.description, t.created_at, t.updated_at, t.member_count, t.post_count, t.visibility
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

pub(super) const UPSERT_TOPIC_METRICS: &str = r#"
    INSERT INTO topic_metrics (
        topic_id,
        window_start,
        window_end,
        posts_24h,
        posts_6h,
        unique_authors,
        boosts,
        replies,
        bookmarks,
        participant_delta,
        score_24h,
        score_6h,
        updated_at
    ) VALUES (
        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13
    )
    ON CONFLICT(topic_id, window_start) DO UPDATE SET
        window_end = excluded.window_end,
        posts_24h = excluded.posts_24h,
        posts_6h = excluded.posts_6h,
        unique_authors = excluded.unique_authors,
        boosts = excluded.boosts,
        replies = excluded.replies,
        bookmarks = excluded.bookmarks,
        participant_delta = excluded.participant_delta,
        score_24h = excluded.score_24h,
        score_6h = excluded.score_6h,
        updated_at = excluded.updated_at
"#;

pub(super) const CLEANUP_TOPIC_METRICS: &str = r#"
    DELETE FROM topic_metrics
    WHERE window_end < ?1
"#;

pub(super) const COLLECT_TOPIC_ACTIVITY: &str = r#"
    SELECT
        et.topic_id AS topic_id,
        COUNT(DISTINCT e.event_id) AS posts_count,
        COUNT(DISTINCT e.public_key) AS unique_authors,
        0 AS boosts,
        0 AS replies,
        0 AS bookmarks,
        0 AS participant_delta
    FROM event_topics et
    INNER JOIN events e ON e.event_id = et.event_id
    WHERE e.kind = 1
      AND e.deleted = 0
      AND e.created_at >= ?1
      AND e.created_at < ?2
    GROUP BY et.topic_id
"#;

pub(super) const SELECT_LATEST_METRICS_WINDOW_END: &str = r#"
    SELECT MAX(window_end) as window_end
    FROM topic_metrics
"#;

pub(super) const SELECT_METRICS_BY_WINDOW: &str = r#"
    SELECT
        topic_id,
        window_start,
        window_end,
        posts_24h,
        posts_6h,
        unique_authors,
        boosts,
        replies,
        bookmarks,
        participant_delta,
        score_24h,
        score_6h,
        updated_at
    FROM topic_metrics
    WHERE window_end = ?1
    ORDER BY score_24h DESC, score_6h DESC, posts_24h DESC, topic_id ASC
    LIMIT ?2
"#;

pub(super) const INSERT_USER: &str = r#"
    INSERT INTO users (
        npub,
        pubkey,
        name,
        display_name,
        bio,
        avatar_url,
        nip05,
        is_profile_public,
        show_online_status,
        created_at,
        updated_at
    )
    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
"#;

pub(super) const SELECT_USER_BY_NPUB: &str = r#"
    SELECT
        npub,
        pubkey,
        name,
        display_name,
        bio,
        avatar_url,
        nip05,
        is_profile_public,
        show_online_status,
        created_at,
        updated_at
    FROM users
    WHERE npub = ?
"#;

pub(super) const SELECT_USER_BY_PUBKEY: &str = r#"
    SELECT
        npub,
        pubkey,
        name,
        display_name,
        bio,
        avatar_url,
        nip05,
        is_profile_public,
        show_online_status,
        created_at,
        updated_at
    FROM users
    WHERE pubkey = ?
"#;

pub(super) const SEARCH_USERS: &str = r#"
    SELECT
        npub,
        pubkey,
        name,
        display_name,
        bio,
        avatar_url,
        nip05,
        is_profile_public,
        show_online_status,
        created_at,
        updated_at
    FROM users
    WHERE display_name LIKE '%' || ?1 || '%' COLLATE NOCASE
       OR npub LIKE '%' || ?1 || '%' COLLATE NOCASE
       OR pubkey LIKE '%' || ?1 || '%' COLLATE NOCASE
       OR bio LIKE '%' || ?1 || '%' COLLATE NOCASE
    ORDER BY updated_at DESC
    LIMIT ?2
"#;

pub(super) const UPDATE_USER: &str = r#"
    UPDATE users
    SET
        name = ?,
        display_name = ?,
        bio = ?,
        avatar_url = ?,
        nip05 = ?,
        is_profile_public = ?,
        show_online_status = ?,
        updated_at = ?
    WHERE npub = ?
"#;

pub(super) const DELETE_USER: &str = r#"
    DELETE FROM users
    WHERE npub = ?
"#;

pub(super) const SELECT_FOLLOWING_PUBKEYS: &str = r#"
    SELECT followed_pubkey
    FROM follows
    WHERE follower_pubkey = ?1
"#;

pub(super) const SELECT_FOLLOWER_PUBKEYS: &str = r#"
    SELECT follower_pubkey
    FROM follows
    WHERE followed_pubkey = ?1
"#;

pub(super) const UPSERT_FOLLOW_RELATION: &str = r#"
    INSERT INTO follows (follower_pubkey, followed_pubkey)
    VALUES (?1, ?2)
    ON CONFLICT(follower_pubkey, followed_pubkey) DO NOTHING
"#;

pub(super) const DELETE_FOLLOW_RELATION: &str = r#"
    DELETE FROM follows
    WHERE follower_pubkey = ?1 AND followed_pubkey = ?2
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
