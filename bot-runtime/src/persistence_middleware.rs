use async_trait::async_trait;
use dbot_core::{HandlerResponse, Message, MessageDirection, Middleware, Result};
use storage::MessageRepository;
use tracing::{debug, error, instrument};
use chrono::Utc;

#[derive(Clone)]
pub struct PersistenceMiddleware {
    repo: MessageRepository,
}

impl PersistenceMiddleware {
    pub fn new(repo: MessageRepository) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl Middleware for PersistenceMiddleware {
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool> {
        debug!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            message_type = %message.message_type,
            "Persisting message"
        );

        let record = storage::MessageRecord::new(
            message.user.id,
            message.chat.id,
            message.user.username.clone(),
            message.user.first_name.clone(),
            message.user.last_name.clone(),
            message.message_type.clone(),
            message.content.clone(),
            match message.direction {
                MessageDirection::Incoming => "received",
                MessageDirection::Outgoing => "sent",
            }
            .to_string(),
        );

        self.repo.save(&record).await.map_err(|e| {
            error!(error = %e, user_id = message.user.id, "Failed to save message");
            dbot_core::DbotError::Database(e.to_string())
        })?;

        debug!(
            user_id = message.user.id,
            message_id = %message.id,
            "Message persisted successfully"
        );

        Ok(true)
    }

    #[instrument(skip(self))]
    async fn after(&self, _message: &Message, _response: &HandlerResponse) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbot_core::{User, Chat};

    fn create_test_message(content: &str) -> Message {
        Message {
            id: "test_message_id".to_string(),
            content: content.to_string(),
            user: User {
                id: 123,
                username: Some("test_user".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 456,
                chat_type: "private".to_string(),
            },
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: Utc::now(),
            reply_to_message_id: None,
        }
    }

    #[tokio::test]
    async fn test_persistence_middleware_creation() {
        let repo = MessageRepository::new("sqlite::memory:")
            .await
            .expect("Failed to create repository");
        let _middleware = PersistenceMiddleware::new(repo);
    }

    #[tokio::test]
    async fn test_persistence_middleware_before() {
        let repo = MessageRepository::new("sqlite::memory:")
            .await
            .expect("Failed to create repository");
        let middleware = PersistenceMiddleware::new(repo.clone());

        let message = create_test_message("Hello");
        let result = middleware.before(&message).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
