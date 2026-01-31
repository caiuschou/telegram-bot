//! # Handlers for dbot framework
//!
//! This crate provides handler implementations: logging, auth, memory, and persistence.

mod logging_auth;
mod memory_handler;
mod persistence_handler;

#[cfg(test)]
mod test;

pub use logging_auth::{AuthHandler, LoggingHandler};
pub use memory_handler::{MemoryConfig, MemoryHandler};
pub use persistence_handler::PersistenceHandler;
