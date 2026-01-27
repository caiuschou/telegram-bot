use async_trait::async_trait;
use dbot_core::{Handler, HandlerResponse, Message, Result};
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Debug, Clone)]
pub struct AIQuery {
    pub chat_id: i64,
    pub user_id: i64,
    pub question: String,
}

#[derive(Clone)]
pub struct AIDetectionHandler {
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    query_sender: Arc<tokio::sync::mpsc::UnboundedSender<AIQuery>>,
}

impl AIDetectionHandler {
    pub fn new(
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        query_sender: Arc<tokio::sync::mpsc::UnboundedSender<AIQuery>>,
    ) -> Self {
        Self {
            bot_username,
            query_sender,
        }
    }

    async fn get_bot_username(&self) -> Option<String> {
        self.bot_username.read().await.clone()
    }

    fn is_bot_mentioned(&self, text: &str, bot_username: &str) -> bool {
        text.contains(&format!("@{}", bot_username))
    }

    fn extract_question(&self, text: &str, bot_username: &str) -> String {
        text.replace(&format!("@{}", bot_username), "")
            .trim()
            .to_string()
    }
}

#[async_trait]
impl Handler for AIDetectionHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        if let Some(bot_username) = self.get_bot_username().await {
            if self.is_bot_mentioned(&message.content, &bot_username) {
                let question = self.extract_question(&message.content, &bot_username);

                if !question.is_empty() {
                    info!(
                        user_id = message.user.id,
                        question = %question,
                        "Bot mentioned, sending to AI queue"
                    );

                    let query = AIQuery {
                        chat_id: message.chat.id,
                        user_id: message.user.id,
                        question,
                    };

                    self.query_sender.send(query).map_err(|e| {
                        dbot_core::DbotError::Bot(format!("Failed to send AI query: {}", e))
                    })?;

                    return Ok(HandlerResponse::Stop);
                }
            }
        }

        Ok(HandlerResponse::Continue)
    }
}
