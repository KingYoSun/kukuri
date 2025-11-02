use aes_gcm::{
    Aes256Gcm, Key,
    aead::{Aead, AeadCore, KeyInit, OsRng, generic_array::GenericArray},
};
use rand::RngCore;

use crate::shared::AppError;

const AES_KEY_SIZE: usize = 32;
const AES_NONCE_SIZE: usize = 12;

/// ストリーム暗号化結果
#[derive(Debug, Clone)]
pub struct StreamEncryptionResult {
    pub ciphertext: Vec<u8>,
    pub session_key: [u8; AES_KEY_SIZE],
    pub nonce: [u8; AES_NONCE_SIZE],
}

/// セッションキーを Capability で暗号化した結果
#[derive(Debug, Clone)]
pub struct EncryptedSessionKey {
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; AES_NONCE_SIZE],
}

/// Blob 本体を暗号化するユーティリティ
pub struct StreamEncryptor;

impl StreamEncryptor {
    pub fn encrypt(plaintext: &[u8]) -> Result<StreamEncryptionResult, AppError> {
        let mut session_key = [0u8; AES_KEY_SIZE];
        OsRng.fill_bytes(&mut session_key);

        let key = Key::<Aes256Gcm>::from_slice(&session_key);
        let cipher = Aes256Gcm::new(key);

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, plaintext)
            .map_err(|err| AppError::Crypto(format!("Failed to encrypt stream: {err}")))?;

        let mut nonce_bytes = [0u8; AES_NONCE_SIZE];
        nonce_bytes.copy_from_slice(&nonce);

        Ok(StreamEncryptionResult {
            ciphertext,
            session_key,
            nonce: nonce_bytes,
        })
    }

    pub fn decrypt(
        ciphertext: &[u8],
        session_key: &[u8; AES_KEY_SIZE],
        nonce: &[u8; AES_NONCE_SIZE],
    ) -> Result<Vec<u8>, AppError> {
        let key = Key::<Aes256Gcm>::from_slice(session_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = GenericArray::from_slice(nonce);
        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|err| AppError::Crypto(format!("Failed to decrypt stream: {err}")))
    }
}

/// Capability 秘密を用いたセッションキー暗号化ユーティリティ
pub struct CapabilityEncryptor;

impl CapabilityEncryptor {
    pub fn encrypt_session_key(
        session_key: &[u8; AES_KEY_SIZE],
        capability_key: &[u8; AES_KEY_SIZE],
    ) -> Result<EncryptedSessionKey, AppError> {
        let key = Key::<Aes256Gcm>::from_slice(capability_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, session_key.as_slice())
            .map_err(|err| AppError::Crypto(format!("Failed to encrypt session key: {err}")))?;

        let mut nonce_bytes = [0u8; AES_NONCE_SIZE];
        nonce_bytes.copy_from_slice(&nonce);

        Ok(EncryptedSessionKey {
            ciphertext,
            nonce: nonce_bytes,
        })
    }

    pub fn decrypt_session_key(
        encrypted: &EncryptedSessionKey,
        capability_key: &[u8; AES_KEY_SIZE],
    ) -> Result<[u8; AES_KEY_SIZE], AppError> {
        let key = Key::<Aes256Gcm>::from_slice(capability_key);
        let cipher = Aes256Gcm::new(key);
        let nonce = GenericArray::from_slice(&encrypted.nonce);
        let decrypted = cipher
            .decrypt(nonce, encrypted.ciphertext.as_slice())
            .map_err(|err| AppError::Crypto(format!("Failed to decrypt session key: {err}")))?;

        if decrypted.len() != AES_KEY_SIZE {
            return Err(AppError::Crypto(
                "Decrypted session key has unexpected length".to_string(),
            ));
        }

        let mut session_key = [0u8; AES_KEY_SIZE];
        session_key.copy_from_slice(&decrypted);
        Ok(session_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_and_decrypt_roundtrip() {
        let data = b"hello profile avatar";
        let encrypted = StreamEncryptor::encrypt(data).expect("encrypt");
        let decrypted = StreamEncryptor::decrypt(
            &encrypted.ciphertext,
            &encrypted.session_key,
            &encrypted.nonce,
        )
        .expect("decrypt");
        assert_eq!(decrypted, data);
    }

    #[test]
    fn capability_encryption_roundtrip() {
        let mut capability_key = [0u8; AES_KEY_SIZE];
        OsRng.fill_bytes(&mut capability_key);
        let mut session_key = [0u8; AES_KEY_SIZE];
        OsRng.fill_bytes(&mut session_key);

        let encrypted = CapabilityEncryptor::encrypt_session_key(&session_key, &capability_key)
            .expect("encrypt");
        let decrypted =
            CapabilityEncryptor::decrypt_session_key(&encrypted, &capability_key).expect("decrypt");
        assert_eq!(session_key, decrypted);
    }
}
