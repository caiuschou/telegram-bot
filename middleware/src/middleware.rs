use async_trait::async_trait;
use dbot_core::{HandlerResponse, Message, Middleware, Result};
use tracing::{debug, error, info, instrument};

pub struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
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

pub struct AuthMiddleware {
    allowed_users: Vec<i64>,
}

impl AuthMiddleware {
    pub fn new(allowed_users: Vec<i64>) -> Self {
        Self { allowed_users }
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
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

    #[instrument(skip(self))]
    async fn after(&self, _message: &Message, _response: &HandlerResponse) -> Result<()> {
        Ok(())
    }
}
