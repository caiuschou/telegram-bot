use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Already exists: {0}")]
    AlreadyExists(String),
}

#[async_trait]
pub trait Repository<T> {
    async fn save(&self, entity: &T) -> Result<(), StorageError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, StorageError>;
    async fn find_all(&self) -> Result<Vec<T>, StorageError>;
    async fn delete(&self, id: &str) -> Result<bool, StorageError>;
}
