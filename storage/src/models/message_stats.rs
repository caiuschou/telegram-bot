//! Aggregate statistics for messages.
//!
//! Returned by MessageRepository::get_stats.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStats {
    pub total_messages: i64,
    pub sent_messages: i64,
    pub received_messages: i64,
    pub unique_users: i64,
    pub unique_chats: i64,
    pub first_message: Option<DateTime<Utc>>,
    pub last_message: Option<DateTime<Utc>>,
}
