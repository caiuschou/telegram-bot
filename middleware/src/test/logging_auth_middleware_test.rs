//! Unit tests for LoggingMiddleware and AuthMiddleware.

use chrono::Utc;
use dbot_core::{Chat, HandlerResponse, Message, MessageDirection, Middleware, User};
use crate::{AuthMiddleware, LoggingMiddleware};

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
async fn test_logging_middleware_before_continues() {
    let mw = LoggingMiddleware;
    let msg = sample_message(1, "hello");
    let result: dbot_core::Result<bool> = mw.before(&msg).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_logging_middleware_after_ok() {
    let mw = LoggingMiddleware;
    let msg = sample_message(1, "hello");
    let response = HandlerResponse::Reply("hi".to_string());
    let result: dbot_core::Result<()> = mw.after(&msg, &response).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_auth_middleware_allowed_user_continues() {
    let mw = AuthMiddleware::new(vec![100, 200]);
    let msg = sample_message(100, "hello");
    let result: dbot_core::Result<bool> = mw.before(&msg).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_auth_middleware_unauthorized_returns_err() {
    let mw = AuthMiddleware::new(vec![100, 200]);
    let msg = sample_message(999, "hello");
    let result: dbot_core::Result<bool> = mw.before(&msg).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        dbot_core::DbotError::Handler(dbot_core::HandlerError::Unauthorized)
    ));
}

#[tokio::test]
async fn test_auth_middleware_after_ok() {
    let mw = AuthMiddleware::new(vec![100]);
    let msg = sample_message(100, "hello");
    let response = HandlerResponse::Stop;
    let result: dbot_core::Result<()> = mw.after(&msg, &response).await;
    assert!(result.is_ok());
}
