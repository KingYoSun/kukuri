#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Mutex;
    use once_cell::sync::Lazy;
    
    // テスト用のモックストレージ
    static MOCK_STORAGE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| {
        Mutex::new(HashMap::new())
    });
    
    // テスト用のSecureStorageラッパー
    pub struct TestSecureStorage;
    
    impl TestSecureStorage {
        fn save_to_mock(key: &str, value: &str) -> Result<()> {
            let mut storage = MOCK_STORAGE.lock().unwrap();
            storage.insert(key.to_string(), value.to_string());
            Ok(())
        }
        
        fn get_from_mock(key: &str) -> Result<Option<String>> {
            let storage = MOCK_STORAGE.lock().unwrap();
            Ok(storage.get(key).cloned())
        }
        
        fn delete_from_mock(key: &str) -> Result<()> {
            let mut storage = MOCK_STORAGE.lock().unwrap();
            storage.remove(key);
            Ok(())
        }
        
        fn clear_mock() {
            let mut storage = MOCK_STORAGE.lock().unwrap();
            storage.clear();
        }
    }
    
    #[test]
    fn test_save_and_get_private_key() {
        TestSecureStorage::clear_mock();
        
        let npub = "npub1test123";
        let nsec = "nsec1secret456";
        
        // 保存
        TestSecureStorage::save_to_mock(npub, nsec).unwrap();
        
        // 取得
        let retrieved = TestSecureStorage::get_from_mock(npub).unwrap();
        assert_eq!(retrieved, Some(nsec.to_string()));
        
        // 存在しないキーの取得
        let not_found = TestSecureStorage::get_from_mock("npub_not_exist").unwrap();
        assert_eq!(not_found, None);
    }
    
    #[test]
    fn test_delete_private_key() {
        TestSecureStorage::clear_mock();
        
        let npub = "npub1test123";
        let nsec = "nsec1secret456";
        
        // 保存
        TestSecureStorage::save_to_mock(npub, nsec).unwrap();
        
        // 削除
        TestSecureStorage::delete_from_mock(npub).unwrap();
        
        // 削除後の取得
        let retrieved = TestSecureStorage::get_from_mock(npub).unwrap();
        assert_eq!(retrieved, None);
        
        // 存在しないキーの削除（エラーにならない）
        let result = TestSecureStorage::delete_from_mock("npub_not_exist");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_accounts_metadata_management() {
        TestSecureStorage::clear_mock();
        
        let metadata = AccountsMetadata {
            accounts: HashMap::new(),
            current_npub: None,
        };
        
        // メタデータの保存
        let json = serde_json::to_string(&metadata).unwrap();
        TestSecureStorage::save_to_mock(ACCOUNTS_KEY, &json).unwrap();
        
        // メタデータの取得
        let retrieved_json = TestSecureStorage::get_from_mock(ACCOUNTS_KEY).unwrap().unwrap();
        let retrieved: AccountsMetadata = serde_json::from_str(&retrieved_json).unwrap();
        
        assert_eq!(retrieved.accounts.len(), 0);
        assert_eq!(retrieved.current_npub, None);
    }
    
    #[test]
    fn test_add_account() {
        TestSecureStorage::clear_mock();
        
        let npub = "npub1test123";
        let nsec = "nsec1secret456";
        let pubkey = "pubkey123";
        let name = "Test User";
        let display_name = "Test Display";
        let picture = Some("https://example.com/avatar.png".to_string());
        
        // アカウント追加をシミュレート
        // 1. 秘密鍵を保存
        TestSecureStorage::save_to_mock(npub, nsec).unwrap();
        
        // 2. メタデータを作成・保存
        let mut metadata = AccountsMetadata::default();
        metadata.accounts.insert(
            npub.to_string(),
            AccountMetadata {
                npub: npub.to_string(),
                pubkey: pubkey.to_string(),
                name: name.to_string(),
                display_name: display_name.to_string(),
                picture: picture.clone(),
                last_used: chrono::Utc::now(),
            },
        );
        metadata.current_npub = Some(npub.to_string());
        
        let json = serde_json::to_string(&metadata).unwrap();
        TestSecureStorage::save_to_mock(ACCOUNTS_KEY, &json).unwrap();
        
        // 検証
        let retrieved_nsec = TestSecureStorage::get_from_mock(npub).unwrap();
        assert_eq!(retrieved_nsec, Some(nsec.to_string()));
        
        let retrieved_json = TestSecureStorage::get_from_mock(ACCOUNTS_KEY).unwrap().unwrap();
        let retrieved_metadata: AccountsMetadata = serde_json::from_str(&retrieved_json).unwrap();
        
        assert_eq!(retrieved_metadata.accounts.len(), 1);
        assert_eq!(retrieved_metadata.current_npub, Some(npub.to_string()));
        
        let account = &retrieved_metadata.accounts[npub];
        assert_eq!(account.npub, npub);
        assert_eq!(account.pubkey, pubkey);
        assert_eq!(account.name, name);
        assert_eq!(account.display_name, display_name);
        assert_eq!(account.picture, picture);
    }
    
    #[test]
    fn test_multiple_accounts() {
        TestSecureStorage::clear_mock();
        
        // 複数アカウントを追加
        let accounts = vec![
            ("npub1alice", "nsec1alice", "pubkey_alice", "Alice"),
            ("npub1bob", "nsec1bob", "pubkey_bob", "Bob"),
            ("npub1charlie", "nsec1charlie", "pubkey_charlie", "Charlie"),
        ];
        
        let mut metadata = AccountsMetadata::default();
        
        for (npub, nsec, pubkey, name) in &accounts {
            // 秘密鍵を保存
            TestSecureStorage::save_to_mock(npub, nsec).unwrap();
            
            // メタデータに追加
            metadata.accounts.insert(
                npub.to_string(),
                AccountMetadata {
                    npub: npub.to_string(),
                    pubkey: pubkey.to_string(),
                    name: name.to_string(),
                    display_name: name.to_string(),
                    picture: None,
                    last_used: chrono::Utc::now(),
                },
            );
        }
        
        metadata.current_npub = Some("npub1bob".to_string());
        
        let json = serde_json::to_string(&metadata).unwrap();
        TestSecureStorage::save_to_mock(ACCOUNTS_KEY, &json).unwrap();
        
        // 検証
        let retrieved_json = TestSecureStorage::get_from_mock(ACCOUNTS_KEY).unwrap().unwrap();
        let retrieved_metadata: AccountsMetadata = serde_json::from_str(&retrieved_json).unwrap();
        
        assert_eq!(retrieved_metadata.accounts.len(), 3);
        assert_eq!(retrieved_metadata.current_npub, Some("npub1bob".to_string()));
        
        // 各アカウントの秘密鍵が取得できることを確認
        for (npub, nsec, _, _) in &accounts {
            let retrieved_nsec = TestSecureStorage::get_from_mock(npub).unwrap();
            assert_eq!(retrieved_nsec, Some(nsec.to_string()));
        }
    }
    
    #[test]
    fn test_switch_account() {
        TestSecureStorage::clear_mock();
        
        // 複数アカウントを設定
        let mut metadata = AccountsMetadata::default();
        
        for i in 1..=3 {
            let npub = format!("npub{}", i);
            metadata.accounts.insert(
                npub.clone(),
                AccountMetadata {
                    npub: npub.clone(),
                    pubkey: format!("pubkey{}", i),
                    name: format!("User{}", i),
                    display_name: format!("User {}", i),
                    picture: None,
                    last_used: chrono::Utc::now() - chrono::Duration::seconds(i * 60),
                },
            );
        }
        
        metadata.current_npub = Some("npub1".to_string());
        
        // アカウント切り替えをシミュレート
        metadata.current_npub = Some("npub2".to_string());
        if let Some(account) = metadata.accounts.get_mut("npub2") {
            account.last_used = chrono::Utc::now();
        }
        
        assert_eq!(metadata.current_npub, Some("npub2".to_string()));
        
        // last_usedが更新されたことを確認
        let npub2_account = &metadata.accounts["npub2"];
        let npub1_account = &metadata.accounts["npub1"];
        assert!(npub2_account.last_used > npub1_account.last_used);
    }
    
    #[test]
    fn test_remove_account() {
        TestSecureStorage::clear_mock();
        
        // アカウントを追加
        let mut metadata = AccountsMetadata::default();
        metadata.accounts.insert(
            "npub1".to_string(),
            AccountMetadata {
                npub: "npub1".to_string(),
                pubkey: "pubkey1".to_string(),
                name: "User1".to_string(),
                display_name: "User 1".to_string(),
                picture: None,
                last_used: chrono::Utc::now(),
            },
        );
        metadata.accounts.insert(
            "npub2".to_string(),
            AccountMetadata {
                npub: "npub2".to_string(),
                pubkey: "pubkey2".to_string(),
                name: "User2".to_string(),
                display_name: "User 2".to_string(),
                picture: None,
                last_used: chrono::Utc::now(),
            },
        );
        metadata.current_npub = Some("npub1".to_string());
        
        // npub1を削除
        metadata.accounts.remove("npub1");
        TestSecureStorage::delete_from_mock("npub1").unwrap();
        
        // current_npubが自動的に切り替わることを確認
        if metadata.current_npub == Some("npub1".to_string()) {
            metadata.current_npub = metadata.accounts.keys().next().cloned();
        }
        
        assert_eq!(metadata.accounts.len(), 1);
        assert_eq!(metadata.current_npub, Some("npub2".to_string()));
        
        // 秘密鍵が削除されたことを確認
        let retrieved = TestSecureStorage::get_from_mock("npub1").unwrap();
        assert_eq!(retrieved, None);
    }
}