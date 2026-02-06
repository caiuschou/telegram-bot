//! User identity type for core messages.

use serde::{Deserialize, Serialize};

/// User identity (id, username, names).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}
