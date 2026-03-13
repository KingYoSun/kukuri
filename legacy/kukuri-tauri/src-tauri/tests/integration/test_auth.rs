// Example integration test for authentication
use crate::common::fixtures;
use crate::common::mocks::{MockKeyManager, MockSecureStorage};

#[tokio::test]
async fn test_create_account() {
    let key_manager = MockKeyManager::new();
    let secure_storage = MockSecureStorage::new();
    
    // Generate keypair
    let keypair = key_manager.generate_keypair().await;
    
    // Store in secure storage
    secure_storage
        .store("current_npub", &keypair.npub)
        .await
        .expect("Failed to store npub");
    
    // Verify storage
    let stored_npub = secure_storage
        .retrieve("current_npub")
        .await
        .expect("Failed to retrieve npub");
    
    assert_eq!(stored_npub, Some(keypair.npub));
}

#[tokio::test]
async fn test_login_with_nsec() {
    let key_manager = MockKeyManager::new();
    let nsec = "nsec1test1234567890abcdefghijklmnopqrstuvwxyz1234567890abcdef";
    
    // Import private key
    let result = key_manager.import_private_key(nsec).await;
    
    assert!(result.is_ok());
    let keypair = result.unwrap();
    assert_eq!(keypair.nsec, nsec);
    
    // Verify it was stored
    let npubs = key_manager.list_npubs().await;
    assert!(npubs.contains(&keypair.npub));
}

#[tokio::test]
async fn test_logout() {
    let secure_storage = MockSecureStorage::new();
    
    // Store npub
    secure_storage
        .store("current_npub", "npub1test")
        .await
        .expect("Failed to store npub");
    
    // Verify it exists
    assert!(secure_storage.exists("current_npub").await);
    
    // Logout (delete)
    secure_storage
        .delete("current_npub")
        .await
        .expect("Failed to delete npub");
    
    // Verify it's gone
    assert!(!secure_storage.exists("current_npub").await);
}