//! # Context Builder
//!
//! This module provides the `ContextBuilder` for constructing AI conversation context
//! from memory store using various strategies.
//!
//! ## Structure
//!
//! - [`types`]: `Context`, `ContextMetadata` and formatting methods
//! - [`builder`]: `ContextBuilder` that orchestrates strategies
//! - [`utils`]: Token estimation and logging helpers
//!
//! ## Example
//!
//! ```rust
//! use memory::context::ContextBuilder;
//! use memory::RecentMessagesStrategy;
//! use memory_inmemory::InMemoryVectorStore;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), anyhow::Error> {
//! let store = Arc::new(InMemoryVectorStore::new());
//! let builder = ContextBuilder::new(store)
//!     .with_token_limit(4096);
//!
//! let context = builder
//!     .for_user("user123")
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```

mod builder;
mod types;
mod utils;

pub use builder::ContextBuilder;
pub use types::{Context, ContextMetadata};
pub use utils::estimate_tokens;

#[cfg(test)]
mod tests;
