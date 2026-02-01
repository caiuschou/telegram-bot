//! Aggregate statistics for messages.
//!
//! Returned by MessageRepository::get_stats.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Aggregate statistics returned by MessageRepository::get_stats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStats {
    /// Total row count in messages table.
    pub total_messages: i64,
    /// Count where direction = 'sent'.
    pub sent_messages: i64,
    /// Count where direction = 'received'.
    pub received_messages: i64,
    /// Count of distinct user_id.
    pub unique_users: i64,
    /// Count of distinct chat_id.
    pub unique_chats: i64,
    /// Earliest created_at (None if table empty).
    pub first_message: Option<DateTime<Utc>>,
    /// Latest created_at (None if table empty).
    pub last_message: Option<DateTime<Utc>>,
}
