//! Core types: user, chat, message, handler response, and Handler trait.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User identity (id, username, names).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

/// Chat (channel or private) identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub chat_type: String,
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

/// Direction of the message (from user or from bot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

/// Handler result for the chain. `Reply(text)` carries the response body so later handlers can use it in `after()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerResponse {
    /// Pass to next handler.
    Continue,
    /// Stop the chain; no response body.
    Stop,
    /// Skip this handler, try next.
    Ignore,
    /// Stop the chain and attach reply text (e.g. save AI response in a handler's `after()`).
    Reply(String),
}

/// Converts a transport-specific user type to core [`User`].
pub trait ToCoreUser: Send + Sync {
    fn to_core(&self) -> User;
}

/// Converts a transport-specific message type to core [`Message`].
pub trait ToCoreMessage: Send + Sync {
    fn to_core(&self) -> Message;
}

/// Single handler concept: optional before / handle / after. Chain runs all before → handle until Stop/Reply → all after (reverse).
#[async_trait]
pub trait Handler: Send + Sync {
    /// Runs before the handle phase. Return false to stop the chain.
    async fn before(&self, _message: &Message) -> crate::error::Result<bool> {
        Ok(true)
    }
    /// Processes the message. Return Stop or Reply to end the handle phase. Default: Continue.
    async fn handle(&self, _message: &Message) -> crate::error::Result<HandlerResponse> {
        Ok(HandlerResponse::Continue)
    }
    /// Runs after the handle phase (reverse order), with the final response.
    async fn after(
        &self,
        _message: &Message,
        _response: &HandlerResponse,
    ) -> crate::error::Result<()> {
        Ok(())
    }
}
