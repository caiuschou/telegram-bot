//! # Memory Crate
//!
//! The `memory` crate provides a flexible and extensible framework for managing conversational memory in the dbot project.
//!
//! ## Features
//!
//! - **Type-safe memory storage** with flexible metadata
//! - **Async trait-based design** for multiple storage backends
//! - **UUID-based identification** for distributed systems
//! - **Serde serialization** for easy data exchange
//!
//! ## Quick Start
//!
//! ```rust
//! use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
//!
//! // Create a memory entry
//! let metadata = MemoryMetadata {
//!     user_id: Some("user123".to_string()),
//!     conversation_id: None,
//!     role: MemoryRole::User,
//!     timestamp: chrono::Utc::now(),
//!     tokens: None,
//!     importance: None,
//! };
//!
//! let entry = MemoryEntry::new("Hello world".to_string(), metadata);
//! ```
//!
//! ## Modules
//!
//! - [`types`] - Core type definitions
//! - [`store`] - Memory storage interface
//! - [`inmemory_store`] - In-memory vector store
//! - [`sqlite_store`] - SQLite-based persistent storage
//! - [`context`] - Context building for conversations
//! - [`strategies`] - Context building strategies
//!
//! ## External Interactions
//!
//! The memory crate interacts with external services:
//! - **Storage backends**: SQLite, Lance, or in-memory storage via `MemoryStore`
//! - **Bot runtime**: Integrates with bot middleware for conversation memory management
//! - **Embedding services**: Uses the `embedding` crate trait for semantic search
//!
//! ## Embedding Integration
//!
//! For text embedding functionality, use the separate crates:
//! - `embedding` - Core `EmbeddingService` trait
//! - `openai-embedding` - OpenAI implementation
//! - `bigmodel-embedding` - BigModel (Zhipu AI) implementation

pub mod types;
pub mod store;
pub mod inmemory_store;
pub mod context;
pub mod strategies;

pub mod migration;

pub use types::*;
pub use store::*;
pub use inmemory_store::InMemoryVectorStore;
pub use context::{Context, ContextBuilder, estimate_tokens};
pub use strategies::{RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy};
