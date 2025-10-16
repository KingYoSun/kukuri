use async_trait::async_trait;
use std::path::PathBuf;

#[async_trait]
pub trait FileStorage: Send + Sync {
    async fn save_file(
        &self,
        path: &PathBuf,
        data: &[u8],
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
    async fn delete_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>>;
    async fn file_exists(&self, path: &PathBuf) -> Result<bool, Box<dyn std::error::Error>>;
    async fn list_files(
        &self,
        directory: &PathBuf,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>>;
    async fn create_directory(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>>;
}
