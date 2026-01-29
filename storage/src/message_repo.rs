//! Message repository: persistence and queries for messages.
//!
//! Uses SqlitePoolManager and the models (MessageRecord, MessageQuery, MessageStats).
//! External: SQLite via sqlx; callers use save/get_messages/get_stats etc.

use crate::models::{MessageQuery, MessageRecord, MessageStats};
use crate::sqlite_pool::SqlitePoolManager;
use chrono::{DateTime, Local, Utc};
use tracing::info;

#[derive(Clone)]
pub struct MessageRepository {
    pool_manager: SqlitePoolManager,
}

impl MessageRepository {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool_manager = SqlitePoolManager::new(database_url).await?;
        let repo = Self { pool_manager };
        repo.init().await?;
        Ok(repo)
    }

    async fn init(&self) -> Result<(), sqlx::Error> {
        info!("Creating database tables if not exist");

        let pool = self.pool_manager.pool();

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                chat_id INTEGER NOT NULL,
                username TEXT,
                first_name TEXT,
                last_name TEXT,
                message_type TEXT NOT NULL,
                content TEXT NOT NULL,
                direction TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_user_id ON messages(user_id);
            CREATE INDEX IF NOT EXISTS idx_messages_chat_id ON messages(chat_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at);
            CREATE INDEX IF NOT EXISTS idx_messages_direction ON messages(direction);
            CREATE INDEX IF NOT EXISTS idx_messages_message_type ON messages(message_type);
            "#,
        )
        .execute(pool)
        .await?;

        info!("Database tables created successfully");
        Ok(())
    }

    pub async fn save(&self, message: &MessageRecord) -> Result<(), sqlx::Error> {
        let pool = self.pool_manager.pool();

        sqlx::query(
            r#"
            INSERT INTO messages (id, user_id, chat_id, username, first_name, last_name, message_type, content, direction, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&message.id)
        .bind(message.user_id)
        .bind(message.chat_id)
        .bind(&message.username)
        .bind(&message.first_name)
        .bind(&message.last_name)
        .bind(&message.message_type)
        .bind(&message.content)
        .bind(&message.direction)
        .bind(message.created_at)
        .execute(pool)
        .await?;

        info!(
            "Saved message: id={}, content={}",
            message.id, message.content
        );
        Ok(())
    }

    pub async fn get_stats(&self) -> Result<MessageStats, sqlx::Error> {
        let pool = self.pool_manager.pool();

        let total_messages: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(pool)
            .await?;

        let sent_messages: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages WHERE direction = 'sent'")
                .fetch_one(pool)
                .await?;

        let received_messages: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM messages WHERE direction = 'received'")
                .fetch_one(pool)
                .await?;

        let unique_users: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT user_id) FROM messages")
            .fetch_one(pool)
            .await?;

        let unique_chats: (i64,) = sqlx::query_as("SELECT COUNT(DISTINCT chat_id) FROM messages")
            .fetch_one(pool)
            .await?;

        let first_message: Option<(DateTime<Utc>,)> =
            sqlx::query_as("SELECT MIN(created_at) FROM messages")
                .fetch_optional(pool)
                .await?;

        let last_message: Option<(DateTime<Utc>,)> =
            sqlx::query_as("SELECT MAX(created_at) FROM messages")
                .fetch_optional(pool)
                .await?;

        Ok(MessageStats {
            total_messages: total_messages.0,
            sent_messages: sent_messages.0,
            received_messages: received_messages.0,
            unique_users: unique_users.0,
            unique_chats: unique_chats.0,
            first_message: first_message.map(|t| t.0),
            last_message: last_message.map(|t| t.0),
        })
    }

    pub async fn get_messages(
        &self,
        query: &MessageQuery,
    ) -> Result<Vec<MessageRecord>, sqlx::Error> {
        let pool = self.pool_manager.pool();
        let mut sql = String::from("SELECT * FROM messages WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(uid) = query.user_id {
            sql.push_str(" AND user_id = ?");
            params.push(uid.to_string());
        }

        if let Some(cid) = query.chat_id {
            sql.push_str(" AND chat_id = ?");
            params.push(cid.to_string());
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        let mut query_builder = sqlx::query_as::<_, MessageRecord>(&sql);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let messages: Vec<MessageRecord> = query_builder.fetch_all(pool).await?;
        info!("Retrieved {} messages", messages.len());

        Ok(messages)
    }

    pub async fn search_messages(
        &self,
        keyword: &str,
        limit: Option<i64>,
    ) -> Result<Vec<MessageRecord>, sqlx::Error> {
        let pool = self.pool_manager.pool();
        let pattern = format!("%{}%", keyword);
        let mut sql =
            "SELECT * FROM messages WHERE content LIKE ? ORDER BY created_at DESC".to_string();

        if let Some(limit_val) = limit {
            sql.push_str(&format!(" LIMIT {}", limit_val));
        }

        let messages = sqlx::query_as(&sql).bind(&pattern).fetch_all(pool).await?;

        info!("Found {} messages matching '{}'", messages.len(), keyword);
        Ok(messages)
    }

    pub async fn cleanup_old_messages(&self, days: i32) -> Result<u64, sqlx::Error> {
        let pool = self.pool_manager.pool();
        let cutoff_date = Local::now() - chrono::Duration::days(days as i64);

        let result = sqlx::query("DELETE FROM messages WHERE created_at < ?")
            .bind(cutoff_date.naive_utc())
            .execute(pool)
            .await?;

        info!(
            "Deleted {} old messages older than {} days",
            result.rows_affected(),
            days
        );
        Ok(result.rows_affected())
    }

    pub async fn get_message_by_id(&self, message_id: &str) -> Result<Option<MessageRecord>, sqlx::Error> {
        let pool = self.pool_manager.pool();

        let message = sqlx::query_as::<_, MessageRecord>("SELECT * FROM messages WHERE id = ?")
            .bind(message_id)
            .fetch_optional(pool)
            .await?;

        Ok(message)
    }

    pub async fn get_recent_messages_by_chat(
        &self,
        chat_id: i64,
        limit: i64,
    ) -> Result<Vec<MessageRecord>, sqlx::Error> {
        let pool = self.pool_manager.pool();

        let messages: Vec<MessageRecord> = sqlx::query_as::<_, MessageRecord>(
            "SELECT * FROM messages WHERE chat_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(chat_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        info!(
            "Retrieved {} recent messages for chat {}",
            messages.len(),
            chat_id
        );

        Ok(messages)
    }
}
