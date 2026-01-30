//! Query parameters for listing/filtering messages.
//!
//! Used by MessageRepository::get_messages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Query parameters for listing/filtering messages in MessageRepository::get_messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQuery {
    /// Filter by Telegram user id.
    pub user_id: Option<i64>,
    /// Filter by chat id.
    pub chat_id: Option<i64>,
    /// Filter by message type (e.g. "text").
    pub message_type: Option<String>,
    /// Filter by direction ("sent" or "received").
    pub direction: Option<String>,
    /// Only messages on or after this time (optional).
    pub start_date: Option<DateTime<Utc>>,
    /// Only messages on or before this time (optional).
    pub end_date: Option<DateTime<Utc>>,
    /// Maximum number of rows to return.
    pub limit: Option<i64>,
    /// Pagination offset (used with limit).
    pub offset: Option<i64>,
}
