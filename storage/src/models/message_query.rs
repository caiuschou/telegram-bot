//! Query parameters for listing/filtering messages.
//!
//! Used by MessageRepository::get_messages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageQuery {
    pub user_id: Option<i64>,
    pub chat_id: Option<i64>,
    pub message_type: Option<String>,
    pub direction: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    /// Pagination offset (used with limit).
    pub offset: Option<i64>,
}
