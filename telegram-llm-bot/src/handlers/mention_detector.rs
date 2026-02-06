//! Detects when the user triggers an LLM query (reply-to-bot or @-mention) and sends an [`LLMQuery`] on a channel for another handler (e.g. SyncLLMHandler) to process.
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

    /// Returns true if `text` contains @bot_username. Delegates to [`telegram_bot::mention::is_bot_mentioned`].
    fn is_bot_mentioned(&self, text: &str, bot_username: &str) -> bool {
        mention::is_bot_mentioned(text, bot_username)
    }

    /// Removes @bot_username from `text` and trims; used as the question for the LLM. Delegates to [`telegram_bot::mention::extract_question`].
    fn extract_question(&self, text: &str, bot_username: &str) -> String {
        mention::extract_question(text, bot_username)
    }
}

#[async_trait]
impl Handler for LLMDetectionHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        // Priority 1: replying to a bot message triggers LLM
        if message.reply_to_message_id.is_some() && message.reply_to_message_from_bot {
            info!(
                user_id = message.user.id,
                reply_to = ?message.reply_to_message_id,
                "Replying to bot message, sending to LLM queue"
            );

            let query = LLMQuery {
                chat_id: message.chat.id,
                user_id: message.user.id,
                question: message.content.clone(),
                reply_to_message_id: message.reply_to_message_id.clone(),
            };

            self.query_sender.send(query).map_err(|e| {
                telegram_bot::DbotError::Bot(format!("Failed to send LLM query: {}", e))
            })?;

            return Ok(HandlerResponse::Stop);
        }

        // Priority 2: @-mention with non-empty question triggers LLM
        if let Some(bot_username) = self.get_bot_username().await {
            if self.is_bot_mentioned(&message.content, &bot_username) {
                let question = self.extract_question(&message.content, &bot_username);
                if !question.is_empty() {
                    info!(
                        user_id = message.user.id,
                        question = %question,
                        "Bot mentioned, sending to LLM queue"
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
            }
        }

        Ok(HandlerResponse::Continue)
    }
}
