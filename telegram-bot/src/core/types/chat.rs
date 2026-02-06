//! Chat identity type for core messages.

use serde::{Deserialize, Serialize};

/// Chat (channel or private) identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub chat_type: String,
}
