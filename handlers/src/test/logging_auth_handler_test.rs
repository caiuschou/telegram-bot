//! Unit tests for LoggingHandler and AuthHandler.

use chrono::Utc;
use telegram_bot::{Chat, HandlerResponse, Message, MessageDirection, Handler, User};
use crate::{AuthHandler, LoggingHandler};

fn sample_message(user_id: i64, content: &str) -> Message {
    Message {
        id: "msg-1".to_string(),
        user: User {
            id: user_id,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: None,
        },
        chat: Chat {
            id: 123,
            chat_type: "private".to_string(),
        },
        content: content.to_string(),
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: Utc::now(),
        reply_to_message_id: None,
        reply_to_message_from_bot: false,
        reply_to_message_content: None,
    }
}

#[tokio::test]
async fn test_logging_handler_before_continues() {
    let h = LoggingHandler;
    let msg = sample_message(1, "hello");
    let result: telegram_bot::Result<bool> = h.before(&msg).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_logging_handler_after_ok() {
    let h = LoggingHandler;
    let msg = sample_message(1, "hello");
    let response = HandlerResponse::Reply("hi".to_string());
    let result: telegram_bot::Result<()> = h.after(&msg, &response).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_auth_handler_allowed_user_continues() {
    let h = AuthHandler::new(vec![100, 200]);
    let msg = sample_message(100, "hello");
    let result: telegram_bot::Result<bool> = h.before(&msg).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_auth_handler_unauthorized_returns_err() {
    let h = AuthHandler::new(vec![100, 200]);
    let msg = sample_message(999, "hello");
    let result: telegram_bot::Result<bool> = h.before(&msg).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        telegram_bot::DbotError::Handler(telegram_bot::HandlerError::Unauthorized)
    ));
}

#[tokio::test]
async fn test_auth_handler_after_ok() {
    let h = AuthHandler::new(vec![100]);
    let msg = sample_message(100, "hello");
    let response = HandlerResponse::Stop;
    let result: telegram_bot::Result<()> = h.after(&msg, &response).await;
    assert!(result.is_ok());
}
