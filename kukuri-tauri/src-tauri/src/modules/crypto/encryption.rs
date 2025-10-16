use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use sha2::{Digest, Sha256};

#[allow(dead_code)]
#[derive(Clone)]
pub struct EncryptionManager;

#[allow(dead_code)]
impl EncryptionManager {
    pub fn new() -> Self {
        Self
    }

    pub fn encrypt(&self, plaintext: &[u8], password: &str) -> Result<String> {
        encrypt(plaintext, password)
    }

    pub fn decrypt(&self, encrypted_data: &str, password: &str) -> Result<Vec<u8>> {
        decrypt(encrypted_data, password)
    }
}

fn encrypt(plaintext: &[u8], password: &str) -> Result<String> {
    // Derive key from password
    let key = derive_key_from_password(password);
    let cipher = Aes256Gcm::new(&key);

    // Generate random nonce
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // Encrypt
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // Combine nonce and ciphertext, then Base64 encode
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(&combined))
}

fn decrypt(encrypted_data: &str, password: &str) -> Result<Vec<u8>> {
    // Base64 decode
    let combined = general_purpose::STANDARD
        .decode(encrypted_data)
        .map_err(|e| anyhow!("Base64 decode failed: {}", e))?;

    // Separate nonce and ciphertext
    if combined.len() < 12 {
        return Err(anyhow!("Invalid encrypted data"));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Derive key from password
    let key = derive_key_from_password(password);
    let cipher = Aes256Gcm::new(&key);

    // Decrypt
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))
}

fn derive_key_from_password(password: &str) -> Key<Aes256Gcm> {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let mut key = Key::<Aes256Gcm>::default();
    key.copy_from_slice(&result);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let plaintext = b"Hello, World! This is a test message.";
        let password = "test_password_123";

        // Encrypt
        let encrypted = encrypt(plaintext, password).unwrap();
        assert!(!encrypted.is_empty());

        // Decrypt
        let decrypted = decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_results() {
        let plaintext = b"Test message";
        let password = "password";

        // Encrypt twice
        let encrypted1 = encrypt(plaintext, password).unwrap();
        let encrypted2 = encrypt(plaintext, password).unwrap();

        // Should be different due to random nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same plaintext
        let decrypted1 = decrypt(&encrypted1, password).unwrap();
        let decrypted2 = decrypt(&encrypted2, password).unwrap();
        assert_eq!(decrypted1, plaintext);
        assert_eq!(decrypted2, plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_password_fails() {
        let plaintext = b"Secret message";
        let password = "correct_password";
        let wrong_password = "wrong_password";

        // Encrypt with correct password
        let encrypted = encrypt(plaintext, password).unwrap();

        // Try to decrypt with wrong password
        let result = decrypt(&encrypted, wrong_password);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Decryption failed")
        );
    }

    #[test]
    fn test_decrypt_invalid_base64_fails() {
        let invalid_base64 = "This is not valid base64!@#$";
        let password = "password";

        let result = decrypt(invalid_base64, password);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Base64 decode failed")
        );
    }

    #[test]
    fn test_decrypt_too_short_data_fails() {
        // Valid base64 but too short (less than 12 bytes for nonce)
        let too_short = general_purpose::STANDARD.encode(b"short");
        let password = "password";

        let result = decrypt(&too_short, password);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Invalid encrypted data");
    }

    #[test]
    fn test_encrypt_empty_plaintext() {
        let plaintext = b"";
        let password = "password";

        // Should be able to encrypt empty data
        let encrypted = encrypt(plaintext, password).unwrap();
        let decrypted = decrypt(&encrypted, password).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_derive_key_consistency() {
        let password = "test_password";

        // Deriving key multiple times with same password should produce same key
        let key1 = derive_key_from_password(password);
        let key2 = derive_key_from_password(password);

        assert_eq!(key1.as_slice(), key2.as_slice());
    }
}
