//! # Lance Vector Store
//!
//! High-performance vector storage using LanceDB.
//!
//! ## Features
//!
//! - **Persistent storage** with Lance format
//! - **Vector indexing** with IVF-PQ and HNSW
//! - **Semantic search** with configurable distance metrics
//! - **Metadata filtering** for efficient querying
//!
//! ## Usage
//!
//! ```rust
//! use memory_lance::LanceVectorStore;
//! use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
//! use chrono::Utc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create store with default settings
//! let store = LanceVectorStore::new("./data/lancedb").await?;
//!
//! // Add entry
//! let metadata = MemoryMetadata {
//!     user_id: Some("user123".to_string()),
//!     conversation_id: None,
//!     role: MemoryRole::User,
//!     timestamp: Utc::now(),
//!     tokens: None,
//!     importance: None,
//! };
//! let entry = MemoryEntry::new("Hello world".to_string(), metadata);
//! store.add(entry).await?;
//!
//! // Semantic search
//! let query_embedding = vec![0.1; 1536]; // OpenAI embedding dimension
//! let results = store.semantic_search(&query_embedding, 10).await?;
//! # Ok(())
//! # }
//! ```

mod config;
mod distance_type;
mod index_type;
mod store;

pub use config::LanceConfig;
pub use distance_type::DistanceType;
pub use index_type::LanceIndexType;
pub use store::LanceVectorStore;
