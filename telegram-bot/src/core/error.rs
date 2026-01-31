//! Error types for the bot core.
//!
//! [`DbotError`] is the top-level error; [`HandlerError`] is used for handler failures.

use thiserror::Error;

/// Top-level error for dbot (database, bot transport, handler, config, IO).
#[derive(Error, Debug)]
pub enum DbotError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Bot error: {0}")]
    Bot(String),

    #[error("Handler error: {0}")]
    Handler(#[from] HandlerError),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Errors produced by handlers (no text, invalid command, auth, state, empty content).
#[derive(Error, Debug)]
pub enum HandlerError {
    #[error("No text in message")]
    NoText,

    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Unauthorized access")]
    Unauthorized,

    #[error("State error: {0}")]
    State(String),

    #[error("Empty content")]
    EmptyContent,
}

/// Result type for core operations; uses [`DbotError`].
pub type Result<T> = std::result::Result<T, DbotError>;
