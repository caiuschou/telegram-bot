//! # Memory Crate
//!
//! The `memory` crate provides a flexible and extensible framework for managing conversational memory in the dbot project.
//!
//! ## Features
//!
//! - **Type-safe memory storage** with flexible metadata
//! - **Async trait-based design** for multiple storage backends
//! - **Embedding service** for semantic search
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
//! - [`embedding`] - Text embedding service
//!
//! ## External Interactions
//!
//! The memory crate interacts with external services:
//! - **OpenAI API**: For generating text embeddings via `EmbeddingService`
//! - **Storage backends**: SQLite, Lance, or in-memory storage via `MemoryStore`
//! - **Bot runtime**: Integrates with bot middleware for conversation memory management

pub mod types;
pub mod store;
pub mod embedding;

pub use types::*;
pub use store::*;
pub use embedding::*;
