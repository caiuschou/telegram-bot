//! # Memory module
//!
//! Abstraction (MemoryStore, types), in-memory and SQLite implementations live here.
//! Lance implementation stays in crates/memory/memory-lance.

pub mod config;
pub mod context;
pub mod inmemory;
pub mod sqlite;

// Re-export abstraction from memory-core
pub use memory_core::*;
// Re-export strategies from memory-strategies
pub use memory_strategies::{
    ContextStrategy, RecentMessagesStrategy, SemanticSearchStrategy, StoreKind,
    UserPreferencesStrategy,
};
// Re-export config and context
pub use config::{EnvMemoryConfig, MemoryConfig};
pub use context::{Context, ContextBuilder, estimate_tokens};
pub use inmemory::InMemoryVectorStore;
pub use sqlite::SQLiteVectorStore;
