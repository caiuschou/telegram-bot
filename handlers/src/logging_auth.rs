//! Handlers for logging and optional auth (allowlist).

use async_trait::async_trait;
use dbot_core::{HandlerResponse, Message, Handler, Result};
use tracing::{debug, error, info, instrument};

/// Logs each message in before() and the response in after(); always continues.
pub struct LoggingHandler;

#[async_trait]
impl Handler for LoggingHandler {
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool> {
        info!(
            user_id = message.user.id,
            username = %message.user.username.as_deref().unwrap_or("unknown"),
            message_content = %message.content,
            "Received message"
        );
        Ok(true)
    }

    #[instrument(skip(self, message, response))]
    async fn after(&self, message: &Message, response: &HandlerResponse) -> Result<()> {
        debug!(
            message_id = ?message.id,
            response = ?response,
            "Processed message"
        );
        Ok(())
    }
}

/// Stops the chain with Unauthorized if message.user.id is not in the allowlist.
pub struct AuthHandler {
    allowed_users: Vec<i64>,
}

impl AuthHandler {
    /// Creates a handler that allows only the given user ids.
    pub fn new(allowed_users: Vec<i64>) -> Self {
        Self { allowed_users }
    }
}

#[async_trait]
impl Handler for AuthHandler {
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool> {
        let user_id = message.user.id;
        if self.allowed_users.contains(&user_id) {
            info!(user_id = user_id, "User authorized");
            Ok(true)
        } else {
            error!(user_id = user_id, "Unauthorized access attempt");
            Err(dbot_core::HandlerError::Unauthorized.into())
        }
    }
}
