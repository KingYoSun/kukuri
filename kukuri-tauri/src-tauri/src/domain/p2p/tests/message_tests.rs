#[cfg(test)]
mod tests {
    use crate::domain::constants::{DEFAULT_PUBLIC_TOPIC_ID, TOPIC_NAMESPACE};
    use crate::domain::entities::TopicVisibility;
    use crate::domain::p2p::message::*;

    #[test]
    fn test_message_type_copy() {
        let msg_type = MessageType::NostrEvent;
        let copied = msg_type;
        assert!(matches!(copied, MessageType::NostrEvent));
    }

    #[test]
    fn test_gossip_message_creation() {
        let msg_type = MessageType::NostrEvent;
        let payload = vec![1, 2, 3, 4, 5];
        let sender = vec![0; 32];

        let message = GossipMessage::new(msg_type, payload.clone(), sender.clone());

        assert!(matches!(message.msg_type, MessageType::NostrEvent));
        assert_eq!(message.payload, payload);
        assert_eq!(message.sender, sender);
        assert!(message.timestamp > 0);
        assert_eq!(message.signature.len(), 0); // 初期状態では署名なし
    }

    #[test]
    fn test_message_id_uniqueness() {
        let messages: Vec<GossipMessage> = (0..100)
            .map(|i| GossipMessage::new(MessageType::Heartbeat, vec![i as u8], vec![0; 32]))
            .collect();

        // すべてのメッセージIDがユニークであることを確認
        let mut ids = messages.iter().map(|m| m.id).collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 100);
    }

    #[test]
    fn test_generate_topic_id() {
        assert_eq!(generate_topic_id("Bitcoin"), "kukuri:tauri:bitcoin");
        assert_eq!(generate_topic_id("NOSTR"), "kukuri:tauri:nostr");
        assert_eq!(generate_topic_id("Test Topic"), "kukuri:tauri:test topic");
        assert_eq!(generate_topic_id("public"), DEFAULT_PUBLIC_TOPIC_ID);
        assert_eq!(
            generate_topic_id("   kukuri:tauri:public   "),
            DEFAULT_PUBLIC_TOPIC_ID
        );
        assert_eq!(generate_topic_id("   "), "kukuri:tauri:default");

        let private = generate_topic_id_with_visibility("secret-room", TopicVisibility::Private);
        assert!(private.starts_with(TOPIC_NAMESPACE));
        let tail = private.trim_start_matches(TOPIC_NAMESPACE);
        assert_eq!(tail.len(), 64);
        assert!(tail.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_topic_id_bytes_respects_visibility() {
        let private = generate_topic_id_with_visibility("hidden", TopicVisibility::Private);
        let private_tail = private.trim_start_matches(TOPIC_NAMESPACE);
        let bytes = topic_id_bytes(&private);
        assert_eq!(hex::encode(bytes), private_tail[..64]);

        let public_bytes = topic_id_bytes(DEFAULT_PUBLIC_TOPIC_ID);
        assert_eq!(public_bytes.len(), 32);
        assert_eq!(
            &public_bytes[..TOPIC_NAMESPACE.len().min(32)],
            &DEFAULT_PUBLIC_TOPIC_ID.as_bytes()[..TOPIC_NAMESPACE.len().min(32)]
        );
    }

    #[test]
    fn test_global_topic_constant() {
        assert_eq!(GLOBAL_TOPIC, "kukuri:global");
    }

    #[test]
    fn test_user_topic_id() {
        let pubkey = "npub1234567890abcdef";
        assert_eq!(user_topic_id(pubkey), "kukuri:user:npub1234567890abcdef");
    }

    #[test]
    fn test_message_to_signing_bytes() {
        let message = GossipMessage::new(MessageType::NostrEvent, vec![1, 2, 3], vec![4, 5, 6]);

        let signing_bytes = message.to_signing_bytes();

        // 署名用バイト列が正しく生成されることを確認
        assert!(!signing_bytes.is_empty());
        assert!(signing_bytes.len() > message.id.len() + message.payload.len());
    }

    #[test]
    fn test_all_message_types() {
        let types = vec![
            MessageType::NostrEvent,
            MessageType::TopicSync,
            MessageType::PeerExchange,
            MessageType::Heartbeat,
        ];

        for msg_type in types {
            let message = GossipMessage::new(msg_type, vec![], vec![0; 32]);
            assert!(matches!(message.msg_type, _));
        }
    }
}
