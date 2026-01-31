//! Storage error types.
//!
//! Used by repository implementations and callers of storage APIs.

use thiserror::Error;

/// Errors that can occur when using storage operations.
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Already exists: {0}")]
    AlreadyExists(String),
}
