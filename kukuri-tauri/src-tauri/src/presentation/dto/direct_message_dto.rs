use serde::{Deserialize, Serialize};

/// 送信要求。暗号化や署名はサービス側で処理する想定。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SendDirectMessageRequest {
    pub recipient_npub: String,
    pub content: String,
    pub client_message_id: Option<String>,
}

/// 送信結果。最小限のメタデータのみ保持。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SendDirectMessageResponse {
    pub event_id: Option<String>,
    pub queued: bool,
}

/// 単一メッセージの DTO。UI 側では `client_message_id` で楽観更新を照合する。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectMessageDto {
    pub event_id: Option<String>,
    pub client_message_id: Option<String>,
    pub sender_npub: String,
    pub recipient_npub: String,
    pub content: String,
    pub created_at: i64,
    pub delivered: bool,
}

/// カーソル付きメッセージ取得リクエスト。
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListDirectMessagesRequest {
    pub conversation_npub: String,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub direction: Option<MessagePageDirection>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListDirectMessageConversationsRequest {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

/// 取得方向。将来的な前方/後方スクロールを想定。
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum MessagePageDirection {
    #[default]
    Backward,
    Forward,
}

/// カーソルページ。`has_more` は UI のロード制御用。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectMessagePage {
    pub items: Vec<DirectMessageDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectMessageConversationSummaryDto {
    pub conversation_npub: String,
    pub unread_count: u64,
    pub last_read_at: i64,
    pub last_message: Option<DirectMessageDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectMessageConversationListDto {
    pub items: Vec<DirectMessageConversationSummaryDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarkDirectMessageConversationReadRequest {
    pub conversation_npub: String,
    pub last_read_at: i64,
}
