//! 单元测试：MemoryMiddleware 的配置、消息与回复写入记忆、上下文构建。
//!
//! 依赖：InMemoryVectorStore；不调用真实外部服务。
//! 与 memory_middleware 的交互：通过 MemoryConfig、MemoryMiddleware 的公开与 pub(crate) 接口进行测试。

use crate::memory_middleware::{MemoryConfig, MemoryMiddleware};
use chrono::Utc;
use dbot_core::{HandlerResponse, Message, Middleware, User, Chat};
use memory::MemoryRole;
use memory_inmemory::InMemoryVectorStore;
use std::sync::Arc;

/// 构造用于测试的 Message，固定 user_id=123、chat_id=456。
fn create_test_message(content: &str) -> Message {
    Message {
        id: "test_message_id".to_string(),
        content: content.to_string(),
        user: User {
            id: 123,
            username: Some("test_user".to_string()),
            first_name: Some("Test".to_string()),
            last_name: None,
        },
        chat: Chat {
            id: 456,
            chat_type: "private".to_string(),
        },
        message_type: "text".to_string(),
        direction: dbot_core::MessageDirection::Incoming,
        created_at: Utc::now(),
        reply_to_message_id: None,
    }
}

#[test]
fn test_memory_config_default() {
    let config = MemoryConfig::default();
    assert_eq!(config.max_recent_messages, 10);
    assert_eq!(config.max_context_tokens, 4096);
    assert!(config.save_user_messages);
    assert!(config.save_ai_responses);
}

#[test]
fn test_memory_middleware_creation() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let middleware = MemoryMiddleware::with_store(store);
    assert!(middleware.config.save_user_messages);
}

#[test]
fn test_message_to_memory_entry() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let middleware = MemoryMiddleware::with_store(store);
    let message = create_test_message("Test message");

    let entry = middleware.message_to_memory_entry(&message);

    assert_eq!(entry.content, "Test message");
    assert_eq!(entry.metadata.role, MemoryRole::User);
    assert_eq!(entry.metadata.user_id, Some("123".to_string()));
    assert_eq!(entry.metadata.conversation_id, Some("456".to_string()));
}

#[tokio::test]
async fn test_memory_middleware_saves_user_messages() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let middleware = MemoryMiddleware::with_store(store.clone());
    let message = create_test_message("Hello");

    middleware.before(&message).await.unwrap();

    let entries = store.search_by_user("123").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "Hello");
}

#[tokio::test]
async fn test_memory_middleware_after_handler_response() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let middleware = MemoryMiddleware::with_store(store.clone());
    let message = create_test_message("Hello");

    let response = HandlerResponse::Continue;

    middleware.after(&message, &response).await.unwrap();

    // Continue carries no reply text, so nothing is saved
    let entries = store.search_by_user("123").await.unwrap();
    assert_eq!(entries.len(), 0);
}

#[tokio::test]
async fn test_memory_middleware_after_saves_reply_to_memory() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let middleware = MemoryMiddleware::with_store(store.clone());
    let message = create_test_message("Hello");

    let response = HandlerResponse::Reply("AI reply here.".to_string());

    middleware.after(&message, &response).await.unwrap();

    let entries = store.search_by_user("123").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "AI reply here.");
    assert_eq!(entries[0].metadata.role, MemoryRole::Assistant);
}

#[tokio::test]
async fn test_memory_middleware_builds_context() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let middleware = MemoryMiddleware::with_store(store.clone());

    let message = create_test_message("Hello");
    middleware.before(&message).await.unwrap();

    let context = middleware.build_context("123", "456").await.unwrap();

    assert!(context.is_some());
    assert!(context.unwrap().contains("Hello"));
}
