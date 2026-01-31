//! Unit tests for MemoryHandler: config, saving user messages and LLM replies to memory.
//!
//! Uses InMemoryVectorStore; no real external services. Tests via MemoryConfig and MemoryHandler public/pub(crate) APIs.

use crate::memory_handler::{MemoryConfig, MemoryHandler};
use chrono::Utc;
use dbot_core::{HandlerResponse, Message, Handler, User, Chat};
use memory::MemoryRole;
use memory_inmemory::InMemoryVectorStore;
use std::sync::Arc;

/// Builds a test Message with fixed user_id=123, chat_id=456.
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
        reply_to_message_from_bot: false,
        reply_to_message_content: None,
    }
}

/// **Test: MemoryConfig::default() has expected max_recent_messages, max_context_tokens, save flags.**
#[test]
fn test_memory_config_default() {
    let config = MemoryConfig::default();
    assert_eq!(config.max_recent_messages, 10);
    assert_eq!(config.max_context_tokens, 4096);
    assert!(config.save_user_messages);
    assert!(config.save_llm_responses);
}

/// **Test: MemoryHandler::with_store(store) creates handler with save_user_messages true.**
#[test]
fn test_memory_handler_creation() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = MemoryHandler::with_store(store);
    assert!(handler.config.save_user_messages);
}

#[test]
fn test_message_to_memory_entry() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = MemoryHandler::with_store(store);
    let message = create_test_message("Test message");

    let entry = handler.message_to_memory_entry(&message);

    assert_eq!(entry.content, "Test message");
    assert_eq!(entry.metadata.role, MemoryRole::User);
    assert_eq!(entry.metadata.user_id, Some("123".to_string()));
    assert_eq!(entry.metadata.conversation_id, Some("456".to_string()));
}

/// **Test: before() saves incoming user message to store; search_by_user returns one entry with User role.**
#[tokio::test]
async fn test_memory_handler_saves_user_messages() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = MemoryHandler::with_store(store.clone());
    let message = create_test_message("Hello");

    handler.before(&message).await.unwrap();

    let entries = store.search_by_user("123").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "Hello");
    assert_eq!(entries[0].metadata.role, memory::MemoryRole::User);
}

/// **Test: after() with HandlerResponse::Continue does not save any entry (no reply text).**
#[tokio::test]
async fn test_memory_handler_after_handler_response() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = MemoryHandler::with_store(store.clone());
    let message = create_test_message("Hello");

    let response = HandlerResponse::Continue;

    handler.after(&message, &response).await.unwrap();

    // Continue carries no reply text, so nothing is saved
    let entries = store.search_by_user("123").await.unwrap();
    assert_eq!(entries.len(), 0);
}

#[tokio::test]
async fn test_memory_handler_after_saves_reply_to_memory() {
    let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
    let handler = MemoryHandler::with_store(store.clone());
    let message = create_test_message("Hello");

    let response = HandlerResponse::Reply("AI reply here.".to_string());

    handler.after(&message, &response).await.unwrap();

    let entries = store.search_by_user("123").await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, "AI reply here.");
    assert_eq!(entries[0].metadata.role, MemoryRole::Assistant);
}
