//! Message record model for persistence.
//!
//! Maps to the `messages` table and is used by MessageRepository.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One row from the messages table; used for save and query results.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageRecord {
    /// Primary key (e.g. UUID).
    pub id: String,
    /// Telegram user id.
    pub user_id: i64,
    /// Chat id.
    pub chat_id: i64,
    /// Telegram username.
    pub username: Option<String>,
    /// User first name.
    pub first_name: Option<String>,
    /// User last name.
    pub last_name: Option<String>,
    /// Message type (e.g. "text").
    pub message_type: String,
    /// Message body.
    pub content: String,
    /// "sent" or "received".
    pub direction: String,
    /// When the message was stored.
    pub created_at: DateTime<Utc>,
    /// Telegram message id (when persisted from Telegram); enables query/dedup by transport id.
    pub telegram_message_id: Option<String>,
}

impl MessageRecord {
    /// Creates a new record with a generated UUID and current timestamp.
    /// Set `telegram_message_id` when the record comes from a Telegram message so it can be queried or deduplicated by transport id.
    pub fn new(
        user_id: i64,
        chat_id: i64,
        username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        message_type: String,
        content: String,
        direction: String,
        telegram_message_id: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            chat_id,
            username,
            first_name,
            last_name,
            message_type,
            content,
            direction,
            created_at: Utc::now(),
            telegram_message_id,
        }
    }
}
