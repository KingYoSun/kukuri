use crate::domain::p2p::message::GossipMessage;

#[derive(Clone, Debug)]
pub enum P2PEvent {
    MessageReceived {
        topic_id: String,
        message: GossipMessage,
        _from_peer: Vec<u8>,
    },
    PeerJoined {
        topic_id: String,
        peer_id: Vec<u8>,
    },
    PeerLeft {
        topic_id: String,
        peer_id: Vec<u8>,
    },
}
