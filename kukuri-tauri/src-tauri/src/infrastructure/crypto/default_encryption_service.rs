use super::encryption_service::EncryptionService;
use crate::shared::error::AppError;
use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};
use std::str;

const NONCE_SIZE: usize = 12;

pub struct DefaultEncryptionService;

impl DefaultEncryptionService {
    pub fn new() -> Self {
        Self
    }

    fn derive_key(password: &str) -> Key<Aes256Gcm> {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let result = hasher.finalize();
        let mut key = Key::<Aes256Gcm>::default();
        key.copy_from_slice(&result);
        key
    }

    fn encrypt_internal(plaintext: &[u8], password: &str) -> Result<Vec<u8>, AppError> {
        let key = Self::derive_key(password);
        let cipher = Aes256Gcm::new(&key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|err| AppError::Crypto(format!("Encryption failed: {err}")))?;

        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);

        Ok(general_purpose::STANDARD.encode(combined).into_bytes())
    }

    fn decrypt_internal(encrypted_data: &[u8], password: &str) -> Result<Vec<u8>, AppError> {
        let encoded = str::from_utf8(encrypted_data)
            .map_err(|err| AppError::Crypto(format!("Invalid encrypted payload: {err}")))?;
        let combined = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|err| AppError::Crypto(format!("Base64 decode failed: {err}")))?;

        if combined.len() < NONCE_SIZE {
            return Err(AppError::Crypto(
                "Encrypted data is shorter than nonce size".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_SIZE);
        let mut nonce = Nonce::default();
        nonce.copy_from_slice(nonce_bytes);

        let key = Self::derive_key(password);
        let cipher = Aes256Gcm::new(&key);

        cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|err| AppError::Crypto(format!("Decryption failed: {err}")))
    }
}

impl Default for DefaultEncryptionService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EncryptionService for DefaultEncryptionService {
    async fn encrypt(
        &self,
        _data: &[u8],
        _recipient_pubkey: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Err(Box::new(AppError::NotImplemented(
            "Asymmetric encryption is not implemented".to_string(),
        )))
    }

    async fn decrypt(
        &self,
        _encrypted_data: &[u8],
        _sender_pubkey: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Err(Box::new(AppError::NotImplemented(
            "Asymmetric decryption is not implemented".to_string(),
        )))
    }

    async fn encrypt_symmetric(
        &self,
        data: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Self::encrypt_internal(data, password).map_err(|err| Box::new(err) as _)
    }

    async fn decrypt_symmetric(
        &self,
        encrypted_data: &[u8],
        password: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        Self::decrypt_internal(encrypted_data, password).map_err(|err| Box::new(err) as _)
    }
}
