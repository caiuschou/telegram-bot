//! # Context Builder
//!
//! This module provides the `ContextBuilder` for constructing AI conversation context
//! from memory store using various strategies.

mod builder;
mod types;
mod utils;

pub use builder::ContextBuilder;
pub use types::{Context, ContextMetadata};
pub use utils::estimate_tokens;

#[cfg(test)]
mod tests;
