//! 单元测试：PersistenceMiddleware 的创建与 before 持久化行为。
//!
//! 依赖：内存 SQLite（sqlite::memory:）；不依赖外部数据库。
//! 与 persistence_middleware 的交互：通过 PersistenceMiddleware 公开接口进行测试。

use crate::persistence_middleware::PersistenceMiddleware;
use dbot_core::{Message, MessageDirection, Middleware};
use storage::MessageRepository;
use chrono::Utc;

/// 构造用于测试的 Message，固定 user_id=123、chat_id=456。
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
    }
}

#[tokio::test]
async fn test_persistence_middleware_creation() {
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("Failed to create repository");
    let _middleware = PersistenceMiddleware::new(repo);
}

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
