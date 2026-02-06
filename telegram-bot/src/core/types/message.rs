//! Message and direction types for the core model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{chat::Chat, user::User};

/// Direction of the message (from user or from bot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

/// A single message with user, chat, content, and optional reply context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub user: User,
    pub chat: Chat,
    pub content: String,
    pub message_type: String,
    pub direction: MessageDirection,
    pub created_at: DateTime<Utc>,
    pub reply_to_message_id: Option<String>,
    /// Whether the replied-to message was sent by the bot; only meaningful when `reply_to_message_id` is set; used to decide if AI should respond.
    pub reply_to_message_from_bot: bool,
    /// Content of the replied-to message; used as context in AI requests so the model knows what the user is replying to.
    pub reply_to_message_content: Option<String>,
}
