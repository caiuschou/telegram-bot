//! # Context Strategies
//!
//! This crate provides strategies for building conversation context from memory store.
//!
//! Available strategies:
//! - `RecentMessagesStrategy`: Retrieves most recent messages
//! - `SemanticSearchStrategy`: Performs semantic search for relevant messages
//! - `UserPreferencesStrategy`: Extracts user preferences from history
//!
//! ## Logging
//!
//! Strategies emit `tracing` debug logs so that memory behavior can be
//! inspected in production:
//! - Selected retrieval path (by conversation / by user / empty)
//! - Number of entries/messages returned
//! - Whether user preferences were detected
//!
//! ## External Interactions
//!
//! - **memory-core**: Uses MemoryStore, MemoryEntry, MemoryRole, StrategyResult
//! - **embedding**: EmbeddingService for semantic search query embedding
//!

mod recent_messages;
mod semantic_search;
mod strategy;
mod user_preferences;
mod utils;

pub use recent_messages::RecentMessagesStrategy;
pub use semantic_search::SemanticSearchStrategy;
pub use strategy::ContextStrategy;
pub use user_preferences::UserPreferencesStrategy;

#[cfg(test)]
mod strategies_test;
