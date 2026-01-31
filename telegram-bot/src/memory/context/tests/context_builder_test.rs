use crate::memory::context::*;
use crate::memory::{ContextStrategy, MemoryEntry, MemoryStore, MessageCategory, StrategyResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use async_trait::async_trait;

struct MockStore {
    entries: Arc<RwLock<HashMap<Uuid, MemoryEntry>>>,
}

impl MockStore {
    fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MemoryStore for MockStore {
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries.get(&id).cloned())
    }

    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.remove(&id);
        Ok(())
    }

    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect())
    }

    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
            .cloned()
            .collect())
    }

    async fn semantic_search(
        &self,
        _query_embedding: &[f32],
        _limit: usize,
        _user_id: Option<&str>,
        _conversation_id: Option<&str>,
    ) -> Result<Vec<(f32, MemoryEntry)>, anyhow::Error> {
        Ok(Vec::new())
    }
}

struct MockStrategy;

#[async_trait::async_trait]
impl ContextStrategy for MockStrategy {
    fn name(&self) -> &str {
        "MockStrategy"
    }

    async fn build_context(
        &self,
        _store: &dyn MemoryStore,
        _user_id: &Option<String>,
        _conversation_id: &Option<String>,
        _query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        Ok(StrategyResult::Messages {
            category: MessageCategory::Recent,
            messages: vec![
                "User: Hello".to_string(),
                "Assistant: Hi there!".to_string(),
            ],
        })
    }
}

#[tokio::test]
async fn test_context_builder_creation() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let builder = ContextBuilder::new(store).with_token_limit(2048);

    assert_eq!(builder.token_limit, 2048);
}

#[tokio::test]
async fn test_context_builder_with_user() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let builder = ContextBuilder::new(store).for_user("user123");

    assert_eq!(builder.user_id.as_deref(), Some("user123"));
}

#[tokio::test]
async fn test_context_builder_for_conversation() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let builder = ContextBuilder::new(store).for_conversation("conv456");

    assert_eq!(builder.conversation_id.as_deref(), Some("conv456"));
}

#[tokio::test]
async fn test_context_builder_with_query() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let builder = ContextBuilder::new(store).with_query("search query");

    assert_eq!(builder.query.as_deref(), Some("search query"));
}

#[tokio::test]
async fn test_context_builder_with_strategies() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let strategy = Box::new(MockStrategy);

    let builder = ContextBuilder::new(store).with_strategy(strategy);

    assert_eq!(builder.strategies.len(), 1);
}

#[tokio::test]
async fn test_context_builder_with_system_message() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let builder = ContextBuilder::new(store).with_system_message("You are a helpful assistant.");

    assert_eq!(
        builder.system_message.as_deref(),
        Some("You are a helpful assistant.")
    );
}

#[tokio::test]
async fn test_context_builder_build_aggregates_strategy_result() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let strategy = Box::new(MockStrategy);

    let context = ContextBuilder::new(store)
        .with_strategy(strategy)
        .for_user("u1")
        .for_conversation("c1")
        .with_system_message("System")
        .build()
        .await
        .expect("build should succeed");

    assert_eq!(context.recent_messages.len(), 2);
    assert!(context.recent_messages[0].contains("Hello"));
    assert!(context.recent_messages[1].contains("Hi there"));
    assert!(context.semantic_messages.is_empty());
    assert_eq!(context.system_message.as_deref(), Some("System"));
    assert_eq!(context.metadata.user_id.as_deref(), Some("u1"));
    assert_eq!(context.metadata.conversation_id.as_deref(), Some("c1"));
    assert_eq!(context.metadata.message_count, 2);
    assert!(context.metadata.total_tokens > 0);
}
