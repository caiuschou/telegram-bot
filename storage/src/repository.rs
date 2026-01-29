//! Repository trait for generic storage operations.
//!
//! Implementations (e.g. MessageRepository) provide concrete persistence.

use async_trait::async_trait;

use crate::error::StorageError;

#[async_trait]
pub trait Repository<T> {
    async fn save(&self, entity: &T) -> Result<(), StorageError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, StorageError>;
    async fn find_all(&self) -> Result<Vec<T>, StorageError>;
    async fn delete(&self, id: &str) -> Result<bool, StorageError>;
}
