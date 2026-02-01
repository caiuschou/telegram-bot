//! Repository trait for generic storage operations. Implementations (e.g. [`MessageRepository`]) provide concrete persistence.

use async_trait::async_trait;

use super::error::StorageError;

/// Generic async repository: save, find_by_id, find_all, delete.
#[async_trait]
pub trait Repository<T> {
    /// Persists the entity; idempotency is implementation-defined.
    async fn save(&self, entity: &T) -> Result<(), StorageError>;
    /// Returns the entity with the given id, or None if not found.
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, StorageError>;
    /// Returns all entities (order is implementation-defined).
    async fn find_all(&self) -> Result<Vec<T>, StorageError>;
    /// Deletes the entity with the given id; returns true if a row was deleted.
    async fn delete(&self, id: &str) -> Result<bool, StorageError>;
}
