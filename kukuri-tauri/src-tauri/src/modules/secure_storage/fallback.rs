use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 開発環境用のフォールバック実装
/// 本番環境では使用しないこと（セキュリティリスクあり）
pub struct FallbackStorage;

impl FallbackStorage {
    fn get_storage_path() -> PathBuf {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("kukuri-dev");
        path.push("secure_storage");
        path
    }

    fn ensure_storage_dir() -> Result<()> {
        let path = Self::get_storage_path();
        fs::create_dir_all(&path)?;
        Ok(())
    }

    pub fn save_data(key: &str, value: &str) -> Result<()> {
        Self::ensure_storage_dir()?;
        let mut path = Self::get_storage_path();
        path.push(format!("{}.json", key));
        
        println!("FallbackStorage: Saving to {:?}", path);
        fs::write(&path, value)?;
        Ok(())
    }

    pub fn get_data(key: &str) -> Result<Option<String>> {
        let mut path = Self::get_storage_path();
        path.push(format!("{}.json", key));
        
        if path.exists() {
            println!("FallbackStorage: Reading from {:?}", path);
            let data = fs::read_to_string(&path)?;
            Ok(Some(data))
        } else {
            println!("FallbackStorage: File not found: {:?}", path);
            Ok(None)
        }
    }

    pub fn delete_data(key: &str) -> Result<()> {
        let mut path = Self::get_storage_path();
        path.push(format!("{}.json", key));
        
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}