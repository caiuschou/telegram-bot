//! SQLite connection pool wrapper for the storage crate.

use log::info;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

/// Manages a single SQLite pool; creates DB file if missing.
#[derive(Clone)]
pub struct SqlitePoolManager {
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
