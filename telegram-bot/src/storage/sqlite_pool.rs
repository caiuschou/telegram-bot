//! SQLite connection pool wrapper for the storage module.
//!
//! Provides [`SqlitePoolManager`] to create and reuse a single pool per database URL;
//! the database file is created if it does not exist.

use log::info;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

/// Manages a single SQLite pool; creates the database file if missing.
#[derive(Clone)]
pub struct SqlitePoolManager {
    /// Underlying sqlx pool for executing queries.
    pool: SqlitePool,
}

impl SqlitePoolManager {
    /// Creates a pool for the given database URL (file path or in-memory).
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        info!("Initializing SQLite pool: {}", database_url);

        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(database_url);

        let pool = SqlitePool::connect_with(options).await?;

        Ok(Self { pool })
    }

    /// Returns the underlying pool for running queries.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
