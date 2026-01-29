//! Storage crate: message persistence and repository abstractions.
//!
//! ## Modules
//!
//! - [`error`] – Storage error types
//! - [`models`] – MessageRecord, MessageQuery, MessageStats
//! - [`repository`] – Repository trait
//! - [`message_repo`] – MessageRepository (SQLite)
//! - [`sqlite_pool`] – SqlitePoolManager

mod error;
mod message_repo;
mod models;
mod repository;
mod sqlite_pool;

#[cfg(test)]
mod message_repo_test;

pub use error::StorageError;
pub use message_repo::MessageRepository;
pub use models::{MessageQuery, MessageRecord, MessageStats};
pub use repository::Repository;
pub use sqlite_pool::SqlitePoolManager;
