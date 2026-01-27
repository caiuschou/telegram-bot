use log::info;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

#[derive(Clone)]
pub struct SqlitePoolManager {
    pool: SqlitePool,
}

impl SqlitePoolManager {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        info!("Initializing SQLite pool: {}", database_url);

        let options = SqliteConnectOptions::new()
            .create_if_missing(true)
            .filename(database_url);

        let pool = SqlitePool::connect_with(options).await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
