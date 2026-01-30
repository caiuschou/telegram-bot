//! # Memory Core
//!
//! Core types and traits for memory storage and context strategies.
//! Used by the `memory` crate and `memory-strategies` crate.
//!
//! ## Modules
//!
//! - [`types`] - MemoryEntry, MemoryMetadata, MemoryRole
//! - [`store`] - MemoryStore trait
//! - [`strategy_result`] - StrategyResult enum (return type of context strategies)

pub mod types;
pub mod store;
pub mod strategy_result;

pub use types::*;
pub use store::*;
pub use strategy_result::*;
