//! # Memory Storage
//!
//! This module defines the memory storage interface for storing and retrieving memory entries.
//!
//! The `MemoryStore` trait is implemented by storage backends (in-memory, SQLite, Lance, etc.).

use async_trait::async_trait;
use uuid::Uuid;

use crate::types::MemoryEntry;

/// Trait for storing and retrieving memory entries.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    /// Adds a new memory entry to the store.
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>;

    /// Retrieves a memory entry by its UUID. Returns `None` if not found.
    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error>;

    /// Updates an existing memory entry.
    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>;

    /// Deletes a memory entry by its UUID.
    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error>;

    /// Retrieves all memory entries for a specific user.
    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error>;

    /// Retrieves all memory entries for a specific conversation.
    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error>;

    /// Performs semantic search using vector embeddings. Returns the top `limit` most similar entries.
    /// When `user_id` or `conversation_id` is provided, only entries matching those filters are considered.
    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        user_id: Option<&str>,
        conversation_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error>;
}
