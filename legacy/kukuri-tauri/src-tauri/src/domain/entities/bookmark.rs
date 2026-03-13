use crate::domain::value_objects::{BookmarkId, EventId, PublicKey};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// ユーザーが投稿をブックマークした履歴を表現するドメインエンティティ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    id: BookmarkId,
    user_pubkey: PublicKey,
    post_id: EventId,
    created_at: DateTime<Utc>,
}

impl Bookmark {
    /// 現在時刻で新しいブックマークを作成する。
    pub fn new(user_pubkey: PublicKey, post_id: EventId) -> Self {
        Self {
            id: BookmarkId::random(),
            user_pubkey,
            post_id,
            created_at: Utc::now(),
        }
    }

    /// 既存レコードからブックマークを復元する。
    pub fn from_parts(
        id: BookmarkId,
        user_pubkey: PublicKey,
        post_id: EventId,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_pubkey,
            post_id,
            created_at,
        }
    }

    pub fn id(&self) -> &BookmarkId {
        &self.id
    }

    pub fn user_pubkey(&self) -> &PublicKey {
        &self.user_pubkey
    }

    pub fn post_id(&self) -> &EventId {
        &self.post_id
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
