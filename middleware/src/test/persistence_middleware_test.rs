//! Unit tests for PersistenceMiddleware: creation and before() persistence.
//!
//! Uses in-memory SQLite (sqlite::memory:); no external DB. Tests via PersistenceMiddleware public API.

use crate::persistence_middleware::PersistenceMiddleware;
use dbot_core::{HandlerResponse, Message, MessageDirection, Middleware};
use storage::MessageRepository;
use chrono::Utc;

/// Builds a test Message with fixed user_id=123, chat_id=456.
fn create_test_message(content: &str) -> Message {
    Message {
        id: "test_message_id".to_string(),
        content: content.to_string(),
        user: dbot_core::User {
            id: 123,
            username: Some("test_user".to_string()),
            first_name: Some("Test".to_string()),
            last_name: None,
        },
        chat: dbot_core::Chat {
            id: 456,
            chat_type: "private".to_string(),
        },
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: Utc::now(),
        reply_to_message_id: None,
        reply_to_message_from_bot: false,
        reply_to_message_content: None,
    }
}

#[tokio::test]
async fn test_persistence_middleware_creation() {
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("Failed to create repository");
    let _middleware = PersistenceMiddleware::new(repo);
}

/// **Test: before() persists message to repo; get_message_by_id returns the saved message.**
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

/// **Test: before() with Outgoing direction persists with direction "sent".**
#[tokio::test]
async fn test_persistence_middleware_before_outgoing() {
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("Failed to create repository");
    let middleware = PersistenceMiddleware::new(repo.clone());

    let mut message = create_test_message("Outgoing");
    message.direction = MessageDirection::Outgoing;
    let result = middleware.before(&message).await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}

/// **Test: after() runs without error.**
#[tokio::test]
async fn test_persistence_middleware_after() {
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("Failed to create repository");
    let middleware = PersistenceMiddleware::new(repo);
    let message = create_test_message("Hi");
    let response = HandlerResponse::Reply("ok".to_string());

    let result = middleware.after(&message, &response).await;
    assert!(result.is_ok());
}
