//! Tests for [`ChatScopedStore`] and [`get_store`]: semantic_search is fixed to chat_id,
//! add/update set conversation_id, and search_by_conversation uses chat_id.

use std::sync::Arc;

use chrono::Utc;
use telegram_bot::memory::{
    get_store, MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore,
};

mod mock_memory_store;
use mock_memory_store::MockMemoryStore;

fn metadata(conversation_id: &str) -> MemoryMetadata {
    MemoryMetadata {
        user_id: Some("u1".to_string()),
        conversation_id: Some(conversation_id.to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    }
}

#[tokio::test]
async fn semantic_search_uses_scoped_chat_id_ignores_caller_conversation_id() {
    let mock = MockMemoryStore::new();
    let inner = Arc::new(mock) as Arc<dyn MemoryStore>;

    inner
        .add(MemoryEntry::new("in chat1".to_string(), metadata("chat1")))
        .await
        .unwrap();
    inner
        .add(MemoryEntry::new("in chat2".to_string(), metadata("chat2")))
        .await
        .unwrap();

    let scoped = get_store(inner.clone(), "chat1");
    let results = scoped
        .semantic_search(&[], 10, None, Some("other"))
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.content, "in chat1");
    assert_eq!(results[0].1.metadata.conversation_id.as_deref(), Some("chat1"));
}

#[tokio::test]
async fn add_sets_conversation_id_to_scoped_chat_id() {
    let mock = MockMemoryStore::new();
    let inner = Arc::new(mock) as Arc<dyn MemoryStore>;
    let scoped = get_store(inner.clone(), "my_chat");

    let mut meta = metadata("ignored");
    meta.conversation_id = Some("ignored".to_string());
    let entry = MemoryEntry::new("content".to_string(), meta);
    let id = entry.id;

    scoped.add(entry).await.unwrap();
    let got = inner.get(id).await.unwrap().unwrap();
    assert_eq!(got.metadata.conversation_id.as_deref(), Some("my_chat"));
    assert_eq!(got.content, "content");
}

#[tokio::test]
async fn search_by_conversation_uses_scoped_chat_id() {
    let mock = MockMemoryStore::new();
    let inner = Arc::new(mock) as Arc<dyn MemoryStore>;
    inner
        .add(MemoryEntry::new("a".to_string(), metadata("c1")))
        .await
        .unwrap();
    inner
        .add(MemoryEntry::new("b".to_string(), metadata("c2")))
        .await
        .unwrap();

    let scoped = get_store(inner, "c1");
    let list = scoped.search_by_conversation("c2").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].content, "a");
    assert_eq!(list[0].metadata.conversation_id.as_deref(), Some("c1"));
}
