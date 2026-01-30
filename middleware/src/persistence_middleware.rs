use async_trait::async_trait;
use dbot_core::{HandlerResponse, Message, MessageDirection, Middleware, Result};
use storage::MessageRepository;
use tracing::{error, info, instrument};

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
        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            message_id = %message.id,
            message_type = %message.message_type,
            "step: PersistenceMiddleware before, saving message"
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

        info!(
            user_id = message.user.id,
            message_id = %message.id,
            "step: PersistenceMiddleware before done, message saved"
        );

        Ok(true)
    }

    #[instrument(skip(self))]
    async fn after(&self, message: &Message, _response: &HandlerResponse) -> Result<()> {
        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            "step: PersistenceMiddleware after"
        );
        Ok(())
    }
}
