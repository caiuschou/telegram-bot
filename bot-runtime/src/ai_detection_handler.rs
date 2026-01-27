use async_trait::async_trait;
use dbot_core::{Handler, HandlerResponse, Message, Result};
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Debug, Clone)]
pub struct AIQuery {
    pub chat_id: i64,
    pub user_id: i64,
    pub question: String,
    pub reply_to_message_id: Option<String>,
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
                        reply_to_message_id: message.reply_to_message_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use dbot_core::{Chat, Message, MessageDirection, User};

    #[test]
    fn test_ai_query_creation() {
        let query = AIQuery {
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
    fn test_ai_query_without_reply_to() {
        let query = AIQuery {
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
        let handler = AIDetectionHandler::new(
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
        let handler = AIDetectionHandler::new(
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
        let handler = AIDetectionHandler::new(
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
            reply_to_message_id: Some("msg456".to_string()),
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Stop)));

        let received_query = rx.recv().await.unwrap();
        assert_eq!(received_query.chat_id, 123);
        assert_eq!(received_query.user_id, 456);
        assert_eq!(received_query.question, "hello world");
        assert_eq!(received_query.reply_to_message_id, Some("msg456".to_string()));
    }

    #[tokio::test]
    async fn test_handler_without_bot_mention() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = AIDetectionHandler::new(
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
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Continue)));
    }

    #[tokio::test]
    async fn test_handler_with_empty_bot_username() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = AIDetectionHandler::new(
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
        };

        let result = handler.handle(&message).await;
        assert!(matches!(result, Ok(HandlerResponse::Continue)));
    }
}
