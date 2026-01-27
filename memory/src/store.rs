//! # Memory Storage
//!
//! This module defines the memory storage interface for storing and retrieving memory entries.
//!
//! ## MemoryStore Trait
//!
//! The `MemoryStore` trait defines the interface for storing and retrieving memory entries.
//!
//! ### Required Methods
//!
//! #### `add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>`
//!
//! Adds a new memory entry to the store.
//!
//! #### `get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error>`
//!
//! Retrieves a memory entry by its UUID. Returns `None` if not found.
//!
//! #### `update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>`
//!
//! Updates an existing memory entry.
//!
//! #### `delete(&self, id: Uuid) -> Result<(), anyhow::Error>`
//!
//! Deletes a memory entry by its UUID.
//!
//! #### `search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error>`
//!
//! Retrieves all memory entries for a specific user.
//!
//! #### `search_by_conversation(&self, conversation_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error>`
//!
//! Retrieves all memory entries for a specific conversation.
//!
//! #### `semantic_search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<MemoryEntry>, anyhow::Error>`
//!
//! Performs semantic search using vector embeddings. Returns the top `limit` most similar entries.
//!
//! ### Implementations
//!
//! #### Planned Implementations
//!
//! - **InMemoryVectorStore**: Simple in-memory storage for testing and development
//! - **SQLiteVectorStore**: Persistent storage using SQLite
//! - **LanceVectorStore**: High-performance vector database (future)
//!
//! ### Example Usage
//!
//! ```rust
//! use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
//! use uuid::Uuid;
//!
//! async fn example(store: &impl MemoryStore) -> Result<(), anyhow::Error> {
//!     // Add an entry
//!     let metadata = MemoryMetadata {
//!         user_id: Some("user123".to_string()),
//!         conversation_id: None,
//!         role: MemoryRole::User,
//!         timestamp: chrono::Utc::now(),
//!         tokens: None,
//!         importance: None,
//!     };
//!     let entry = MemoryEntry::new("Hello world".to_string(), metadata);
//!     let entry_id = entry.id;
//!
//!     store.add(entry).await?;
//!
//!     // Get by ID
//!     let found = store.get(entry_id).await?;
//!
//!     // Search by user
//!     let entries = store.search_by_user("user123").await?;
//!
//!     // Semantic search
//!     let embedding = vec![0.1, 0.2, 0.3]; // Obtained from embedding service
//!     let similar = store.semantic_search(&embedding, 10).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Storage Considerations
//!
//! ### Performance
//!
//! - **In-Memory**: Fastest, but data is lost on restart
//! - **SQLite**: Good balance of performance and persistence
//! - **Lance**: Optimized for vector search at scale
//!
//! ### Scalability
//!
//! - **Small Scale (< 10K entries)**: SQLite or in-memory
//! - **Medium Scale (10K - 100K entries)**: SQLite with proper indexing
//! - **Large Scale (> 100K entries)**: Lance or dedicated vector database
//!
//! ### Migration
//!
//! The crate will provide migration tools to transfer data between storage backends.

use async_trait::async_trait;
use crate::types::MemoryEntry;
use uuid::Uuid;

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
    async fn search_by_conversation(&self, conversation_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error>;
    
    /// Performs semantic search using vector embeddings. Returns the top `limit` most similar entries.
    async fn semantic_search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<MemoryEntry>, anyhow::Error>;
}
