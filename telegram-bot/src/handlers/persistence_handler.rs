//! Handler that persists incoming messages to storage in before().

use crate::core::{Handler, HandlerResponse, Message, MessageDirection, Result};
use async_trait::async_trait;
use crate::storage::MessageRepository;
use tracing::{error, info, instrument};

/// Saves each incoming message to the given [`MessageRepository`] in before(); always continues.
#[derive(Clone)]
pub struct PersistenceHandler {
    repo: MessageRepository,
}

impl PersistenceHandler {
    /// Creates a handler that persists messages with the given repository.
    pub fn new(repo: MessageRepository) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl Handler for PersistenceHandler {
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool> {
        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            message_id = %message.id,
            message_type = %message.message_type,
            "step: PersistenceHandler before, saving message"
        );

        let record = crate::storage::MessageRecord::new(
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
            crate::core::DbotError::Database(e.to_string())
        })?;

        info!(
            user_id = message.user.id,
            message_id = %message.id,
            "step: PersistenceHandler before done, message saved"
        );

        Ok(true)
    }

    #[instrument(skip(self))]
    async fn after(&self, message: &Message, _response: &HandlerResponse) -> Result<()> {
        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            "step: PersistenceHandler after"
        );
        Ok(())
    }
}
