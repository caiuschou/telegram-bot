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

//! - [`sqlite_store`] - SQLite-based persistent storage
//! - [`context`] - Context building for conversations
//! - [`memory_strategies`] - Context building strategies (re-exported from memory-strategies crate)
//!
//! ## External Interactions
//!
//! The memory crate interacts with external services:
//! - **Storage backends**: SQLite, Lance, or in-memory storage via `MemoryStore`
//! - **Bot runtime**: Integrates with bot middleware for conversation memory management
//! - **Embedding services**: Strategies use the `embedding` crate trait for semantic search
//!
//! ## Embedding Integration
//!
//! For text embedding functionality, use the separate crates:
//! - `embedding` - Core `EmbeddingService` trait
//! - `openai-embedding` - OpenAI implementation
//! - `bigmodel-embedding` - BigModel (Zhipu AI) implementation

pub mod config;
pub mod context;
pub mod migration;

// Re-export config types
pub use config::{EnvMemoryConfig, MemoryConfig};
// Re-export core types and store trait from memory-core
pub use memory_core::*;
// Re-export strategies and ContextStrategy from memory-strategies
pub use memory_strategies::{
    ContextStrategy, RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy,
};
pub use context::{Context, ContextBuilder, estimate_tokens};
