use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MockKeyManager {
    keypairs: Arc<RwLock<HashMap<String, MockKeyPair>>>,
}

#[derive(Debug, Clone)]
pub struct MockKeyPair {
    pub npub: String,
    pub nsec: String,
    pub public_key: String,
    pub private_key: String,
}

impl MockKeyManager {
    pub fn new() -> Self {
        Self {
            keypairs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn generate_keypair(&self) -> MockKeyPair {
        let id = uuid::Uuid::new_v4().to_string();
        let keypair = MockKeyPair {
            npub: format!("npub1{}", &id[..59]),
            nsec: format!("nsec1{}", &id[..59]),
            public_key: format!("pubkey_{}", id),
            private_key: format!("privkey_{}", id),
        };
        
        self.keypairs.write().await.insert(keypair.npub.clone(), keypair.clone());
        keypair
    }

    pub async fn import_private_key(&self, nsec: &str) -> Result<MockKeyPair, String> {
        if !nsec.starts_with("nsec1") {
            return Err("Invalid nsec format".to_string());
        }
        
        let id = &nsec[5..];
        let keypair = MockKeyPair {
            npub: format!("npub1{}", id),
            nsec: nsec.to_string(),
            public_key: format!("pubkey_{}", id),
            private_key: format!("privkey_{}", id),
        };
        
        self.keypairs.write().await.insert(keypair.npub.clone(), keypair.clone());
        Ok(keypair)
    }

    pub async fn get_keypair(&self, npub: &str) -> Option<MockKeyPair> {
        self.keypairs.read().await.get(npub).cloned()
    }

    pub async fn list_npubs(&self) -> Vec<String> {
        self.keypairs.read().await.keys().cloned().collect()
    }
}

#[derive(Debug, Clone)]
pub struct MockSignatureService;

impl MockSignatureService {
    pub fn new() -> Self {
        Self
    }

    pub async fn sign_event(&self, event_id: &str, _private_key: &str) -> String {
        format!("signature_for_{}", event_id)
    }

    pub async fn verify_event(&self, _event_id: &str, _signature: &str, _public_key: &str) -> bool {
        true // Always return true in mock
    }

    pub async fn sign_message(&self, message: &str, _private_key: &str) -> String {
        format!("sig_{}", &message[..message.len().min(10)])
    }

    pub async fn verify_message(&self, _message: &str, _signature: &str, _public_key: &str) -> bool {
        true // Always return true in mock
    }
}

#[derive(Debug, Clone)]
pub struct MockEncryptionService;

impl MockEncryptionService {
    pub fn new() -> Self {
        Self
    }

    pub async fn encrypt(&self, data: &[u8], _recipient_pubkey: &str) -> Vec<u8> {
        // Simple mock: just reverse the bytes
        data.iter().rev().cloned().collect()
    }

    pub async fn decrypt(&self, encrypted_data: &[u8], _sender_pubkey: &str) -> Vec<u8> {
        // Simple mock: reverse back
        encrypted_data.iter().rev().cloned().collect()
    }

    pub async fn encrypt_symmetric(&self, data: &[u8], password: &str) -> Vec<u8> {
        // Simple mock: XOR with password hash
        let key = password.bytes().cycle();
        data.iter().zip(key).map(|(d, k)| d ^ k).collect()
    }

    pub async fn decrypt_symmetric(&self, encrypted_data: &[u8], password: &str) -> Vec<u8> {
        // Simple mock: XOR back with password hash
        let key = password.bytes().cycle();
        encrypted_data.iter().zip(key).map(|(d, k)| d ^ k).collect()
    }
}

#[derive(Debug, Clone)]
pub struct MockSecureStorage {
    storage: Arc<RwLock<HashMap<String, String>>>,
}

impl MockSecureStorage {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn store(&self, key: &str, value: &str) -> Result<(), String> {
        self.storage.write().await.insert(key.to_string(), value.to_string());
        Ok(())
    }

    pub async fn retrieve(&self, key: &str) -> Result<Option<String>, String> {
        Ok(self.storage.read().await.get(key).cloned())
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.storage.write().await.remove(key);
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> bool {
        self.storage.read().await.contains_key(key)
    }

    pub async fn list_keys(&self) -> Vec<String> {
        self.storage.read().await.keys().cloned().collect()
    }

    pub async fn clear(&self) -> Result<(), String> {
        self.storage.write().await.clear();
        Ok(())
    }
}