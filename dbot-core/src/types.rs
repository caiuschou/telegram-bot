use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub chat_type: String,
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

/// Handler result for the chain. `Reply(text)` carries the response body so middleware (e.g. memory) can use it in `after()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerResponse {
    /// Pass to next handler.
    Continue,
    /// Stop the chain; no response body.
    Stop,
    /// Skip this handler, try next.
    Ignore,
    /// Stop the chain and attach reply text for middleware (e.g. save AI response to memory in `after()`).
    Reply(String),
}

pub trait ToCoreUser: Send + Sync {
    fn to_core(&self) -> User;
}

pub trait ToCoreMessage: Send + Sync {
    fn to_core(&self) -> Message;
}

#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, message: &Message) -> crate::error::Result<HandlerResponse>;
}

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn before(&self, message: &Message) -> crate::error::Result<bool>;
    async fn after(
        &self,
        message: &Message,
        response: &HandlerResponse,
    ) -> crate::error::Result<()>;
}
