use serde_json::json;

pub fn create_test_user(npub: &str) -> serde_json::Value {
    json!({
        "npub": npub,
        "pubkey": format!("pubkey_{}", npub),
        "name": format!("Test User {}", npub),
        "display_name": format!("Test {}", npub),
        "about": "Test user for unit tests",
        "picture": "https://example.com/avatar.jpg",
        "created_at": 1234567890,
        "updated_at": 1234567890
    })
}

pub fn create_test_post(id: &str, author_npub: &str, topic_id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "content": format!("Test post content {}", id),
        "author": create_test_user(author_npub),
        "topic_id": topic_id,
        "created_at": 1234567890,
        "tags": ["test", "fixture"],
        "likes": 0,
        "boosts": 0,
        "replies": [],
        "is_synced": false,
        "is_boosted": false,
        "is_bookmarked": false,
        "local_id": id,
        "event_id": null
    })
}

pub fn create_test_topic(id: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "description": format!("Test topic {}", name),
        "created_at": 1234567890,
        "updated_at": 1234567890,
        "is_joined": false,
        "member_count": 0,
        "post_count": 0,
        "is_public": true,
        "owner": null
    })
}

pub fn create_test_event(id: &str, kind: u32, content: &str, pubkey: &str) -> serde_json::Value {
    json!({
        "id": id,
        "pubkey": pubkey,
        "created_at": 1234567890,
        "kind": kind,
        "tags": [],
        "content": content,
        "sig": format!("signature_{}", id)
    })
}

pub fn create_test_keypair() -> (String, String, String, String) {
    let id = uuid::Uuid::new_v4().to_string();
    let npub = format!("npub1{}", &id[..59]);
    let nsec = format!("nsec1{}", &id[..59]);
    let pubkey = format!("pubkey_{}", id);
    let privkey = format!("privkey_{}", id);
    
    (npub, nsec, pubkey, privkey)
}

pub fn create_test_database_url() -> String {
    ":memory:".to_string()
}

pub fn create_test_config() -> serde_json::Value {
    json!({
        "database": {
            "url": ":memory:",
            "max_connections": 5,
            "connection_timeout": 30
        },
        "network": {
            "bootstrap_peers": [],
            "max_peers": 50,
            "connection_timeout": 30,
            "retry_interval": 60
        },
        "sync": {
            "auto_sync": true,
            "sync_interval": 300,
            "max_retry": 3,
            "batch_size": 100
        },
        "storage": {
            "data_dir": "./test_data",
            "cache_size": 100000000,
            "cache_ttl": 3600
        }
    })
}