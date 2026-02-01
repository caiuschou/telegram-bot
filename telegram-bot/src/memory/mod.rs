//! # Memory module
//!
//! Abstraction (MemoryStore, types), in-memory, SQLite, and Lance implementations.

pub mod config;
pub mod context;
pub mod inmemory;
pub mod sqlite;

pub use crate::memory_core::*;
pub use crate::memory_strategies::{
    ContextStrategy, RecentMessagesStrategy, SemanticSearchStrategy, StoreKind,
    UserPreferencesStrategy,
};
pub use config::{EnvMemoryConfig, MemoryConfig};
pub use context::{Context, ContextBuilder, estimate_tokens};
pub use inmemory::InMemoryVectorStore;
pub use sqlite::SQLiteVectorStore;
