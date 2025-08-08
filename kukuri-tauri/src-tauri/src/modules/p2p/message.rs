use bincode::{Decode, Encode};
use chrono::Utc;
use secp256k1::ecdsa::Signature;
use secp256k1::SECP256K1;
use secp256k1::{Message as Secp256k1Message, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub type MessageId = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct GossipMessage {
    /// メッセージID（重複チェック用）
    pub id: MessageId,

    /// メッセージタイプ
    pub msg_type: MessageType,

    /// ペイロード
    pub payload: Vec<u8>,

    /// タイムスタンプ
    pub timestamp: i64,

    /// 送信者の公開鍵（33バイト - 圧縮形式）
    pub sender: Vec<u8>,

    /// 署名
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
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
    #[allow(dead_code)]
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
        // 注意: senderは署名に含めない（署名作成時にはまだ設定されていないため）
        bytes
    }

    /// バイト列からメッセージを復元
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::decode_from_slice(bytes, bincode::config::standard())
            .map(|(msg, _)| msg)
            .map_err(|e| format!("Failed to deserialize message: {e}"))
    }

    /// メッセージをバイト列に変換
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::encode_to_vec(self, bincode::config::standard())
            .map_err(|e| format!("Failed to serialize message: {e}"))
    }

    /// メッセージに署名を付ける
    pub fn sign(&mut self, secret_key: &SecretKey) -> Result<(), String> {
        let signing_bytes = self.to_signing_bytes();

        // SHA256ハッシュを計算
        let mut hasher = Sha256::new();
        hasher.update(&signing_bytes);
        let hash = hasher.finalize();

        // ハッシュからSecp256k1メッセージを作成
        let message = Secp256k1Message::from_digest_slice(&hash)
            .map_err(|e| format!("Failed to create message: {e}"))?;

        // 署名
        let signature = SECP256K1.sign_ecdsa(&message, secret_key);
        self.signature = signature.serialize_compact().to_vec();

        // 公開鍵を設定（圧縮形式）
        let public_key = PublicKey::from_secret_key(SECP256K1, secret_key);
        self.sender = public_key.serialize().to_vec();

        Ok(())
    }

    /// 署名を検証
    pub fn verify_signature(&self) -> Result<bool, String> {
        if self.signature.is_empty() || self.sender.is_empty() {
            return Ok(false);
        }

        // 公開鍵を復元
        let public_key =
            PublicKey::from_slice(&self.sender).map_err(|e| format!("Invalid public key: {e}"))?;

        // 署名を復元
        let signature = Signature::from_compact(&self.signature)
            .map_err(|e| format!("Invalid signature: {e}"))?;

        // 署名対象のバイト列を作成
        let mut message_for_verification = self.clone();
        message_for_verification.signature = Vec::new(); // 署名フィールドを空にする
        let signing_bytes = message_for_verification.to_signing_bytes();

        // SHA256ハッシュを計算
        let mut hasher = Sha256::new();
        hasher.update(&signing_bytes);
        let hash = hasher.finalize();

        // ハッシュからSecp256k1メッセージを作成
        let message = Secp256k1Message::from_digest_slice(&hash)
            .map_err(|e| format!("Failed to create message: {e}"))?;

        // 署名を検証
        Ok(SECP256K1
            .verify_ecdsa(&message, &signature, &public_key)
            .is_ok())
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
    format!("kukuri:user:{pubkey}")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_signing_and_verification() {
        // 秘密鍵を生成
        let secret_key = SecretKey::new(&mut rand::thread_rng());

        // メッセージを作成
        let mut message = GossipMessage::new(MessageType::NostrEvent, vec![1, 2, 3, 4, 5], vec![]);

        // 署名
        assert!(message.sign(&secret_key).is_ok());
        assert!(!message.signature.is_empty());
        assert!(!message.sender.is_empty());

        // 検証 - 正しい署名
        assert!(message.verify_signature().unwrap());

        // ペイロードを改ざん
        message.payload.push(6);
        assert!(!message.verify_signature().unwrap());

        // 署名を改ざん
        message.payload.pop(); // 元に戻す
        if !message.signature.is_empty() {
            message.signature[0] ^= 0xFF;
        }
        assert!(!message.verify_signature().unwrap());
    }

    #[test]
    fn test_message_serialization() {
        let message = GossipMessage::new(
            MessageType::TopicSync,
            vec![10, 20, 30],
            vec![1; 33], // 公開鍵は33バイト
        );

        // シリアライズ
        let bytes = message.to_bytes().unwrap();

        // デシリアライズ
        let deserialized = GossipMessage::from_bytes(&bytes).unwrap();

        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.msg_type as u8, deserialized.msg_type as u8);
        assert_eq!(message.payload, deserialized.payload);
        assert_eq!(message.timestamp, deserialized.timestamp);
        assert_eq!(message.sender, deserialized.sender);
    }
}
