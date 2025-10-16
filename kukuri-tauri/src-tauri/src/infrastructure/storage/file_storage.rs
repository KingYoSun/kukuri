use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[async_trait]
pub trait FileStorage: Send + Sync {
    async fn save_file(&self, path: &Path, data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    async fn delete_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>>;
    async fn file_exists(&self, path: &Path) -> Result<bool, Box<dyn std::error::Error>>;
    async fn list_files(
        &self,
        directory: &Path,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>;
    async fn create_directory(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>>;
}
