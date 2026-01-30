//! Message record model for persistence.
//!
//! Maps to the `messages` table and is used by MessageRepository.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageRecord {
    pub id: String,
    pub user_id: i64,
    pub chat_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub message_type: String,
    pub content: String,
    pub direction: String,
    pub created_at: DateTime<Utc>,
}

impl MessageRecord {
    /// Creates a new record with a generated UUID and current timestamp.
    pub fn new(
        user_id: i64,
        chat_id: i64,
        username: Option<String>,
        first_name: Option<String>,
        last_name: Option<String>,
        message_type: String,
        content: String,
        direction: String,
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
        }
    }
}
