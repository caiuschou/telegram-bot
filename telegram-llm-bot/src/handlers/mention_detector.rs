//! Detects when the user triggers an LLM query (reply-to-bot or @-mention) and sends an [`LLMQuery`] on a channel for another handler (e.g. SyncLLMHandler) to process.

use super::mention;
use async_trait::async_trait;
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

    /// Returns true if `text` contains @bot_username. Delegates to shared [`mention::is_bot_mentioned`].
    fn is_bot_mentioned(&self, text: &str, bot_username: &str) -> bool {
        mention::is_bot_mentioned(text, bot_username)
    }

    /// Removes @bot_username from `text` and trims; used as the question for the LLM. Delegates to shared [`mention::extract_question`].
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

#[cfg(test)]
mod tests {
    use super::*;
    use telegram_bot::{Chat, Message, MessageDirection, User};

    #[test]
    fn test_llm_query_creation() {
        let query = LLMQuery {
            chat_id: 123,
            user_id: 456,
            question: "What is the weather?".to_string(),
            reply_to_message_id: Some("msg123".to_string()),
        };

        assert_eq!(query.chat_id, 123);
        assert_eq!(query.user_id, 456);
        assert_eq!(query.question, "What is the weather?");
        assert_eq!(query.reply_to_message_id, Some("msg123".to_string()));
    }

    #[test]
    fn test_llm_query_without_reply_to() {
        let query = LLMQuery {
            chat_id: 123,
            user_id: 456,
            question: "Hello".to_string(),
            reply_to_message_id: None,
        };

        assert!(query.reply_to_message_id.is_none());
    }

    #[test]
    fn test_is_bot_mentioned() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(Some("mybot".to_string()))),
            Arc::new(tx),
        );

        assert!(handler.is_bot_mentioned("@mybot hello", "mybot"));
        assert!(handler.is_bot_mentioned("Hello @mybot", "mybot"));
        assert!(!handler.is_bot_mentioned("Hello world", "mybot"));
        assert!(!handler.is_bot_mentioned("@otherbot hello", "mybot"));
    }

    #[test]
    fn test_extract_question() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(Some("mybot".to_string()))),
            Arc::new(tx),
        );

        assert_eq!(
            handler.extract_question("@mybot hello world", "mybot"),
            "hello world"
        );
        assert_eq!(
            handler.extract_question("Hello @mybot how are you?", "mybot"),
            "Hello  how are you?"
        );
        assert_eq!(
            handler.extract_question("@mybot  ", "mybot"),
            ""
        );
    }

    #[tokio::test]
    async fn test_handler_with_bot_mention() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(Some("mybot".to_string()))),
            Arc::new(tx),
        );

        let message = Message {
            id: "msg123".to_string(),
            user: User {
                id: 456,
                username: Some("testuser".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 123,
                chat_type: "group".to_string(),
            },
            content: "@mybot hello world".to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
            reply_to_message_id: None,
            reply_to_message_from_bot: false,
            reply_to_message_content: None,
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Stop)));

        let received_query = rx.recv().await.unwrap();
        assert_eq!(received_query.chat_id, 123);
        assert_eq!(received_query.user_id, 456);
        assert_eq!(received_query.question, "hello world");
        assert_eq!(received_query.reply_to_message_id, None);
    }

    #[tokio::test]
    async fn test_handler_without_bot_mention() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(Some("mybot".to_string()))),
            Arc::new(tx),
        );

        let message = Message {
            id: "msg123".to_string(),
            user: User {
                id: 456,
                username: Some("testuser".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 123,
                chat_type: "group".to_string(),
            },
            content: "Hello world".to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
            reply_to_message_id: None,
            reply_to_message_from_bot: false,
            reply_to_message_content: None,
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Continue)));
    }

    #[tokio::test]
    async fn test_handler_with_empty_bot_username() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(None)),
            Arc::new(tx),
        );

        let message = Message {
            id: "msg123".to_string(),
            user: User {
                id: 456,
                username: Some("testuser".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 123,
                chat_type: "group".to_string(),
            },
            content: "@bot hello".to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
            reply_to_message_id: None,
            reply_to_message_from_bot: false,
            reply_to_message_content: None,
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Continue)));
    }

    #[tokio::test]
    async fn test_handler_with_reply_to_bot_message() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(None)),
            Arc::new(tx),
        );

        let message = Message {
            id: "msg123".to_string(),
            user: User {
                id: 456,
                username: Some("testuser".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 123,
                chat_type: "group".to_string(),
            },
            content: "This is a reply to bot".to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
            reply_to_message_id: Some("bot_msg_456".to_string()),
            reply_to_message_from_bot: true,
            reply_to_message_content: Some("Previous bot message".to_string()),
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Stop)));

        let received_query = rx.recv().await.unwrap();
        assert_eq!(received_query.chat_id, 123);
        assert_eq!(received_query.user_id, 456);
        assert_eq!(received_query.question, "This is a reply to bot");
        assert_eq!(received_query.reply_to_message_id, Some("bot_msg_456".to_string()));
    }

    #[tokio::test]
    async fn test_handler_reply_to_non_bot_does_not_trigger() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(Some("mybot".to_string()))),
            Arc::new(tx),
        );

        let message = Message {
            id: "msg123".to_string(),
            user: User {
                id: 456,
                username: Some("testuser".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 123,
                chat_type: "group".to_string(),
            },
            content: "reply to user message".to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
            reply_to_message_id: Some("user_msg_789".to_string()),
            reply_to_message_from_bot: false,
            reply_to_message_content: Some("User's previous message".to_string()),
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Continue)));
    }

    #[tokio::test]
    async fn test_handler_reply_to_bot_takes_priority_over_mention() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = LLMDetectionHandler::new(
            Arc::new(tokio::sync::RwLock::new(Some("mybot".to_string()))),
            Arc::new(tx),
        );

        let message = Message {
            id: "msg123".to_string(),
            user: User {
                id: 456,
                username: Some("testuser".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 123,
                chat_type: "group".to_string(),
            },
            content: "@mybot hello world".to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
            reply_to_message_id: Some("bot_msg_789".to_string()),
            reply_to_message_from_bot: true,
            reply_to_message_content: Some("Previous bot message".to_string()),
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Stop)));

        let received_query = rx.recv().await.unwrap();
        assert_eq!(received_query.question, "@mybot hello world");
        assert_eq!(received_query.reply_to_message_id, Some("bot_msg_789".to_string()));
    }
}
