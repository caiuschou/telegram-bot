//! Detects when the user triggers an LLM query (reply-to-bot or @-mention) and sends an [`LLMQuery`] on a channel for another handler (e.g. InlineLLMHandler) to process.
//! Uses [`telegram_bot::mention`] for @-mention detection and question extraction.

use async_trait::async_trait;
use telegram_bot::mention;
use telegram_bot::{Handler, HandlerResponse, Message, Result};
use std::sync::Arc;
use tracing::{info, instrument};

/// One LLM query to process: chat, user, question text, and optional reply-to message id.
#[derive(Debug, Clone)]
pub struct LLMQuery {
    pub chat_id: i64,
    pub user_id: i64,
    pub question: String,
    pub reply_to_message_id: Option<String>,
}

/// Handler that detects reply-to-bot or @bot_username mention and sends [`LLMQuery`] to `query_sender`.
#[derive(Clone)]
pub struct LLMDetectionHandler {
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    query_sender: Arc<tokio::sync::mpsc::UnboundedSender<LLMQuery>>,
}

impl LLMDetectionHandler {
    /// Creates a handler that sends detected queries to the given channel.
    pub fn new(
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        query_sender: Arc<tokio::sync::mpsc::UnboundedSender<LLMQuery>>,
    ) -> Self {
        Self {
            bot_username,
            query_sender,
        }
    }

    async fn get_bot_username(&self) -> Option<String> {
        self.bot_username.read().await.clone()
    }
}

#[async_trait]
impl Handler for LLMDetectionHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let bot_username = self.get_bot_username().await;
        let question = mention::get_question(
            message,
            bot_username.as_deref(),
            Some(mention::DEFAULT_EMPTY_MENTION_PROMPT),
        );

        if let Some(question) = question {
            info!(
                user_id = message.user.id,
                reply_to = ?message.reply_to_message_id,
                question = %question,
                "LLM query detected, sending to queue"
            );

            let query = LLMQuery {
                chat_id: message.chat.id,
                user_id: message.user.id,
                question,
                reply_to_message_id: message.reply_to_message_id.clone(),
            };

            self.query_sender.send(query).map_err(|e| {
                telegram_bot::DbotError::Bot(format!("Failed to send LLM query: {}", e))
            })?;

            return Ok(HandlerResponse::Stop);
        }

        Ok(HandlerResponse::Continue)
    }
}
