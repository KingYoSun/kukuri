use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

pub type MessageId = [u8; 32];
pub type Signature = Vec<u8>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    /// メッセージID（重複チェック用）
    pub id: MessageId,
    
    /// メッセージタイプ
    pub msg_type: MessageType,
    
    /// ペイロード
    pub payload: Vec<u8>,
    
    /// タイムスタンプ
    pub timestamp: i64,
    
    /// 送信者の公開鍵（32バイト）
    pub sender: Vec<u8>,
    
    /// 署名
    pub signature: Signature,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageType {
    /// Nostrイベント
    NostrEvent,
    
    /// トピック情報の同期
    TopicSync,
    
    /// ピア情報の交換
    PeerExchange,
    
    /// ハートビート
    Heartbeat,
}

impl GossipMessage {
    /// 新しいメッセージを作成
    pub fn new(msg_type: MessageType, payload: Vec<u8>, sender: Vec<u8>) -> Self {
        let id = generate_message_id();
        let timestamp = Utc::now().timestamp();
        
        Self {
            id,
            msg_type,
            payload,
            timestamp,
            sender,
            signature: Vec::new(), // 署名は後で追加
        }
    }
    
    /// メッセージIDを生成
    fn generate_message_id() -> MessageId {
        let uuid = Uuid::new_v4();
        let mut id = [0u8; 32];
        let uuid_bytes = uuid.as_bytes();
        id[..16].copy_from_slice(uuid_bytes);
        id[16..].copy_from_slice(uuid_bytes);
        id
    }
    
    /// メッセージを署名用のバイト列に変換
    pub fn to_signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.id);
        bytes.extend_from_slice(&(self.msg_type as u8).to_le_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.sender);
        bytes
    }
}

/// トピックIDの生成
pub fn generate_topic_id(topic_name: &str) -> String {
    format!("kukuri:topic:{}", topic_name.to_lowercase())
}

/// グローバルトピック（全体のタイムライン）
pub const GLOBAL_TOPIC: &str = "kukuri:global";

/// ユーザー固有トピック
pub fn user_topic_id(pubkey: &str) -> String {
    format!("kukuri:user:{}", pubkey)
}

fn generate_message_id() -> MessageId {
    let uuid = Uuid::new_v4();
    let mut id = [0u8; 32];
    let uuid_bytes = uuid.as_bytes();
    id[..16].copy_from_slice(uuid_bytes);
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    id[16..24].copy_from_slice(&timestamp.to_le_bytes());
    id[24..].copy_from_slice(&uuid_bytes[8..]);
    id
}