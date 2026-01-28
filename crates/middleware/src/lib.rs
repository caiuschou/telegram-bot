//! # Middleware crate for dbot framework
//!
//! This crate provides various middleware implementations for the dbot framework,
//! including logging, authentication, memory management, and persistence middleware.

mod middleware;
mod memory_middleware;
mod persistence_middleware;

pub use middleware::{AuthMiddleware, LoggingMiddleware};
pub use memory_middleware::{MemoryConfig, MemoryMiddleware};
pub use persistence_middleware::PersistenceMiddleware;
