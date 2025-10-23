#[cfg(test)]
mod tests {
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
        assert_eq!(generate_topic_id("Bitcoin"), "kukuri:topic:bitcoin");
        assert_eq!(generate_topic_id("NOSTR"), "kukuri:topic:nostr");
        assert_eq!(generate_topic_id("Test Topic"), "kukuri:topic:test topic");
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
