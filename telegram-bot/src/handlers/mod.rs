//! Handler implementations: persistence, logging, auth, memory. Merged from handlers and memory-handler crates.

mod logging_auth;
mod memory_handler;
mod noop_handler;
mod persistence_handler;

#[cfg(test)]
mod memory_handler_test;

pub use logging_auth::{AuthHandler, LoggingHandler};
pub use memory_handler::{MemoryConfig, MemoryHandler};
pub use noop_handler::NoOpHandler;
pub use persistence_handler::PersistenceHandler;
