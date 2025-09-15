use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::domain::value_objects::EventId;
use sha2::{Digest, Sha256};
use bech32::FromBase32 as _;
use nostr_sdk::prelude::{FromBech32 as _, EventId as NostrEventId, PublicKey as NostrPublicKey};
// NIP-19 厳密なTLVデコードはSDKの公開API差異があるため、
// ここではEventId/PublicKeyのbech32復号と形式検証で代替する。

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[repr(u32)]
pub enum EventKind {
    Metadata = 0,
    TextNote = 1,
    RecommendRelay = 2,
    Contacts = 3,
    EncryptedDirectMessage = 4,
    EventDeletion = 5,
    Repost = 6,
    Reaction = 7,
    BadgeAward = 8,
    ChannelCreation = 40,
    ChannelMetadata = 41,
    ChannelMessage = 42,
    ChannelHideMessage = 43,
    ChannelMuteUser = 44,
    Custom(u32),
}

impl From<u32> for EventKind {
    fn from(value: u32) -> Self {
        match value {
            0 => EventKind::Metadata,
            1 => EventKind::TextNote,
            2 => EventKind::RecommendRelay,
            3 => EventKind::Contacts,
            4 => EventKind::EncryptedDirectMessage,
            5 => EventKind::EventDeletion,
            6 => EventKind::Repost,
            7 => EventKind::Reaction,
            8 => EventKind::BadgeAward,
            40 => EventKind::ChannelCreation,
            41 => EventKind::ChannelMetadata,
            42 => EventKind::ChannelMessage,
            43 => EventKind::ChannelHideMessage,
            44 => EventKind::ChannelMuteUser,
            v => EventKind::Custom(v),
        }
    }
}

impl From<EventKind> for u32 {
    fn from(value: EventKind) -> Self {
        match value {
            EventKind::Metadata => 0,
            EventKind::TextNote => 1,
            EventKind::RecommendRelay => 2,
            EventKind::Contacts => 3,
            EventKind::EncryptedDirectMessage => 4,
            EventKind::EventDeletion => 5,
            EventKind::Repost => 6,
            EventKind::Reaction => 7,
            EventKind::BadgeAward => 8,
            EventKind::ChannelCreation => 40,
            EventKind::ChannelMetadata => 41,
            EventKind::ChannelMessage => 42,
            EventKind::ChannelHideMessage => 43,
            EventKind::ChannelMuteUser => 44,
            EventKind::Custom(v) => v,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: DateTime<Utc>,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl Event {
    pub fn new(kind: u32, content: String, pubkey: String) -> Self {
        Self {
            id: String::new(),
            pubkey,
            created_at: chrono::Utc::now(),
            kind,
            tags: Vec::new(),
            content,
            sig: String::new(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<Vec<String>>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_tag(&mut self, tag: Vec<String>) {
        self.tags.push(tag);
    }

    pub fn add_p_tag(&mut self, pubkey: String) {
        self.tags.push(vec!["p".to_string(), pubkey]);
    }

    pub fn add_e_tag(&mut self, event_id: String) {
        self.tags.push(vec!["e".to_string(), event_id]);
    }

    pub fn add_t_tag(&mut self, hashtag: String) {
        self.tags.push(vec!["t".to_string(), hashtag]);
    }

    pub fn get_referenced_event_ids(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "e")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn get_referenced_pubkeys(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "p")
            .map(|tag| tag[1].clone())
            .collect()
    }

    pub fn get_hashtags(&self) -> Vec<String> {
        self.tags
            .iter()
            .filter(|tag| tag.len() >= 2 && tag[0] == "t")
            .map(|tag| tag[1].clone())
            .collect()
    }
    
    pub fn new_with_id(
        id: EventId,
        pubkey: String,
        content: String,
        kind: u32,
        tags: Vec<Vec<String>>,
        created_at: DateTime<Utc>,
        sig: String,
    ) -> Self {
        Self {
            id: id.to_hex(),
            pubkey,
            created_at,
            kind,
            tags,
            content,
            sig,
        }
    }

    /// NIP-01に基づく基本バリデーション
    /// - idは[0,pubkey,created_at,kind,tags,content]のsha256
    /// - pubkeyは32byte hex（64桁）
    /// - sigは64byte hex（128桁）
    pub fn validate_nip01(&self) -> Result<(), String> {
        // 1) 形式検証
        let is_hex = |s: &str| s.chars().all(|c| c.is_ascii_hexdigit());
        if self.pubkey.len() != 64 || !is_hex(&self.pubkey) {
            return Err("invalid pubkey (expect 64 hex)".into());
        }
        if self.sig.len() != 128 || !is_hex(&self.sig) {
            return Err("invalid sig (expect 128 hex)".into());
        }
        if self.id.len() != 64 || !is_hex(&self.id) {
            return Err("invalid id (expect 64 hex)".into());
        }

        // 2) ID再計算
        // created_atは秒
        let created_at_secs = self.created_at.timestamp();
        // JSON array を構築
        let arr = serde_json::json!([
            0,
            self.pubkey,
            created_at_secs,
            self.kind,
            self.tags,
            self.content,
        ]);
        let serialized = serde_json::to_vec(&arr).map_err(|e| format!("serialization error: {}", e))?;
        let hash = Sha256::digest(&serialized);
        let calc_id = format!("{:x}", hash);
        if calc_id != self.id {
            return Err("id mismatch (not NIP-01 compliant)".into());
        }
        Ok(())
    }

    /// NIP-10/NIP-19 の基本バリデーション
    /// - e タグ: イベント参照（64hex または note1... のbech32）。markerはroot/reply/mentionのみ。root/replyは多重不可。
    /// - p タグ: 公開鍵参照（64hex または npub1... のbech32）
    pub fn validate_nip10_19(&self) -> Result<(), String> {
        let mut root_seen = 0usize;
        let mut reply_seen = 0usize;

        for tag in &self.tags {
            if tag.is_empty() { continue; }
            match tag[0].as_str() {
                "e" => {
                    if tag.len() < 2 { return Err("invalid e tag (len < 2)".into()); }
                    let evref = &tag[1];
                    if !(is_hex_n(evref, 64) || is_valid_event_ref(evref)) {
                        return Err("invalid e tag id (not hex or bech32)".into());
                    }
                    // 推奨リレーURLチェック（3番目要素）
                    if tag.len() >= 3 {
                        let relay_url = tag[2].as_str();
                        if !relay_url.is_empty() && !is_ws_url(relay_url) {
                            return Err("invalid e tag relay_url (expect ws[s]://)".into());
                        }
                    }
                    // markerチェック（4番目要素）
                    if tag.len() >= 4 {
                        let marker = tag[3].as_str();
                        match marker {
                            "root" => root_seen += 1,
                            "reply" => reply_seen += 1,
                            "mention" => {},
                            _ => return Err(format!("invalid e tag marker: {}", marker)),
                        }
                    }
                }
                "p" => {
                    if tag.len() < 2 { return Err("invalid p tag (len < 2)".into()); }
                    let pkref = &tag[1];
                    if !(is_hex_n(pkref, 64) || is_valid_pubkey_ref(pkref)) {
                        return Err("invalid p tag pubkey (not hex or bech32)".into());
                    }
                    // 推奨リレーURLチェック（3番目要素）
                    if tag.len() >= 3 {
                        let relay_url = tag[2].as_str();
                        if !relay_url.is_empty() && !is_ws_url(relay_url) {
                            return Err("invalid p tag relay_url (expect ws[s]://)".into());
                        }
                    }
                }
                _ => {}
            }
        }

        if root_seen > 1 { return Err("multiple root markers in e tags".into()); }
        if reply_seen > 1 { return Err("multiple reply markers in e tags".into()); }
        Ok(())
    }
}

fn is_hex_n(s: &str, n: usize) -> bool {
    s.len() == n && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_valid_event_ref(s: &str) -> bool {
    // 支持: note1... / nevent1...
    if s.starts_with("note1") {
        return NostrEventId::from_bech32(s).is_ok();
    }
    if s.starts_with("nevent1") {
        return is_valid_nevent_tlv(s);
    }
    false
}

fn is_valid_pubkey_ref(s: &str) -> bool {
    // 支持: npub1... / nprofile1...
    if s.starts_with("npub1") { return NostrPublicKey::from_bech32(s).is_ok(); }
    if s.starts_with("nprofile1") { return is_valid_nprofile_tlv(s); }
    false
}

fn is_ws_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    (lower.starts_with("ws://") || lower.starts_with("wss://")) && lower.len() > 5
}

fn is_valid_nprofile_tlv(s: &str) -> bool {
    if let Ok((hrp, data, _)) = bech32::decode(s) {
        if hrp != "nprofile" { return false; }
        if let Ok(bytes) = Vec::<u8>::from_base32(&data) {
            // 必須: tag=0 (pubkey 32bytes)
            if !tlv_has_tag_len(&bytes, 0, 32) { return false; }
            // 任意: tag=1 (relay URL), 複数可。存在する場合はws[s]://であること
            if !tlv_validate_relays(&bytes) { return false; }
            return true;
        }
    }
    false
}

fn is_valid_nevent_tlv(s: &str) -> bool {
    if let Ok((hrp, data, _)) = bech32::decode(s) {
        if hrp != "nevent" { return false; }
        if let Ok(bytes) = Vec::<u8>::from_base32(&data) {
            if !tlv_has_tag_len(&bytes, 0, 32) { return false; }
            if !tlv_validate_relays(&bytes) { return false; }
            return true;
        }
    }
    false
}

fn tlv_has_tag_len(bytes: &[u8], want_tag: u8, want_len: usize) -> bool {
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let t = bytes[i];
        let l = bytes[i + 1] as usize;
        i += 2;
        if i + l > bytes.len() { return false; }
        if t == want_tag && l == want_len { return true; }
        i += l;
    }
    false
}


impl EventKind {
    pub fn as_u32(&self) -> u32 {
        u32::from(*self)
    }

    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(EventKind::Metadata),
            1 => Some(EventKind::TextNote),
            2 => Some(EventKind::RecommendRelay),
            3 => Some(EventKind::Contacts),
            4 => Some(EventKind::EncryptedDirectMessage),
            5 => Some(EventKind::EventDeletion),
            6 => Some(EventKind::Repost),
            7 => Some(EventKind::Reaction),
            8 => Some(EventKind::BadgeAward),
            40 => Some(EventKind::ChannelCreation),
            41 => Some(EventKind::ChannelMetadata),
            42 => Some(EventKind::ChannelMessage),
            43 => Some(EventKind::ChannelHideMessage),
            44 => Some(EventKind::ChannelMuteUser),
            _ => None,
        }
    }
}

fn tlv_validate_relays(bytes: &[u8]) -> bool {
    let mut i = 0usize;
    while i + 2 <= bytes.len() {
        let t = bytes[i];
        let l = bytes[i + 1] as usize;
        i += 2;
        if i + l > bytes.len() { return false; }
        if t == 1 {
            let slice = &bytes[i..i + l];
            if let Ok(s) = std::str::from_utf8(slice) {
                if !s.is_empty() && !is_ws_url(s) { return false; }
            } else {
                return false;
            }
        }
        i += l;
    }
    true
}

#[cfg(test)]
mod tests {
    use nostr_sdk::prelude::*;

    #[tokio::test]
    async fn test_validate_nip01_ok() {
        let keys = Keys::generate();
        let nostr_ev = EventBuilder::text_note("hello nip01")
            .sign_with_keys(&keys)
            .unwrap();

        let created_at = chrono::DateTime::<chrono::Utc>::from_utc(
            chrono::NaiveDateTime::from_timestamp_opt(nostr_ev.created_at.as_u64() as i64, 0).unwrap(),
            chrono::Utc,
        );

        let dom = super::Event {
            id: nostr_ev.id.to_string(),
            pubkey: nostr_ev.pubkey.to_string(),
            created_at,
            kind: nostr_ev.kind.as_u16() as u32,
            tags: nostr_ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
            content: nostr_ev.content.clone(),
            sig: nostr_ev.sig.to_string(),
        };

        assert!(dom.validate_nip01().is_ok());
    }

    #[tokio::test]
    async fn test_validate_nip01_bad_id() {
        let keys = Keys::generate();
        let nostr_ev = EventBuilder::text_note("oops")
            .sign_with_keys(&keys)
            .unwrap();

        let created_at = chrono::DateTime::<chrono::Utc>::from_utc(
            chrono::NaiveDateTime::from_timestamp_opt(nostr_ev.created_at.as_u64() as i64, 0).unwrap(),
            chrono::Utc,
        );

        let mut dom = super::Event {
            id: nostr_ev.id.to_string(),
            pubkey: nostr_ev.pubkey.to_string(),
            created_at,
            kind: nostr_ev.kind.as_u16() as u32,
            tags: nostr_ev.tags.iter().map(|t| t.clone().to_vec()).collect(),
            content: nostr_ev.content.clone(),
            sig: nostr_ev.sig.to_string(),
        };
        // 改ざん
        dom.content = "tampered".into();
        assert!(dom.validate_nip01().is_err());
    }
}

#[cfg(test)]
mod nip10_19_tests {
    use nostr_sdk::prelude::*;

    fn dummy_event_with_tags(tags: Vec<Vec<String>>) -> super::Event {
        super::Event {
            id: "0".repeat(64),
            pubkey: "f".repeat(64),
            created_at: chrono::Utc::now(),
            kind: 1,
            tags,
            content: String::new(),
            sig: "f".repeat(128),
        }
    }

    #[test]
    fn test_validate_nip10_19_ok_with_bech32_refs() {
        let keys = Keys::generate();
        let npub = keys.public_key().to_bech32().unwrap();

        // 参照用のイベントID
        let nostr_ev = EventBuilder::text_note("x").sign_with_keys(&keys).unwrap();
        let note = nostr_ev.id.to_bech32().unwrap();

        let e_tag = vec!["e".into(), note, String::new(), "reply".into()];
        let p_tag = vec!["p".into(), npub];
        let ev = dummy_event_with_tags(vec![e_tag, p_tag]);
        assert!(ev.validate_nip10_19().is_ok());
    }

    #[test]
    fn test_validate_nip10_19_rejects_invalid_marker_and_pk() {
        let e_tag = vec!["e".into(), "0".repeat(64), String::new(), "bad".into()];
        let p_tag = vec!["p".into(), "zzz".into()];
        let ev = dummy_event_with_tags(vec![e_tag, p_tag]);
        assert!(ev.validate_nip10_19().is_err());
    }
}
