//! Unit tests for context strategies.
//!
//! Tests RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy
//! and helper functions (format_message, extract_preferences).

use super::*;
use async_trait::async_trait;
use embedding::EmbeddingService;
use memory_core::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, StrategyResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;

/// Mock embedding service for tests: returns a fixed-dimension vector.
struct MockEmbeddingService;

#[async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, anyhow::Error> {
        Ok(vec![0.0; 1536])
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        Ok(texts.iter().map(|_| vec![0.0; 1536]).collect())
    }
}

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
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries.values().take(limit).cloned().collect())
    }
}

#[test]
fn test_recent_messages_strategy_creation() {
    let _ = RecentMessagesStrategy::new(5);
}

#[tokio::test]
async fn test_recent_messages_by_conversation() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
    let strategy = RecentMessagesStrategy::new(10);

    let user_id = Some("user123".to_string());
    let conversation_id = Some("conv1".to_string());

    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: Some("conv1".to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };

    let entry1 = MemoryEntry::new("Hello".to_string(), metadata.clone());
    let entry2 = MemoryEntry::new("How are you?".to_string(), metadata);

    store.add(entry1).await.unwrap();
    store.add(entry2).await.unwrap();

    let result = strategy
        .build_context(&*store, &user_id, &conversation_id, &None)
        .await
        .unwrap();

    match result {
        StrategyResult::Messages(msgs) => {
            assert_eq!(msgs.len(), 2);
            let combined = msgs.join(" ");
            assert!(combined.contains("How are you?"));
            assert!(combined.contains("Hello"));
        }
        _ => panic!("Expected Messages result"),
    }
}

#[test]
fn test_semantic_search_strategy_creation() {
    let _ = SemanticSearchStrategy::new(5, Arc::new(MockEmbeddingService));
}

#[tokio::test]
async fn test_semantic_search_no_query() {
    let store: Arc<dyn MemoryStore> = Arc::new(MockStore::new());
    let strategy = SemanticSearchStrategy::new(5, Arc::new(MockEmbeddingService));

    let result = strategy
        .build_context(&*store, &None, &None, &None)
        .await
        .unwrap();

    assert!(matches!(result, StrategyResult::Empty));
}

#[test]
fn test_user_preferences_strategy_creation() {
    let _ = UserPreferencesStrategy::new();
}

#[tokio::test]
async fn test_user_preferences_extraction() {
    let store: Arc<dyn MemoryStore> = Arc::new(MockStore::new());

    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };

    let entry = MemoryEntry::new("I like pizza and I prefer tea".to_string(), metadata);
    store.add(entry).await.unwrap();

    let strategy = UserPreferencesStrategy::new();
    let user_id = Some("user123".to_string());

    let result = strategy
        .build_context(&*store, &user_id, &None, &None)
        .await
        .unwrap();

    match result {
        StrategyResult::Preferences(prefs) => {
            assert!(prefs.contains("like"));
        }
        _ => panic!("Expected Preferences result"),
    }
}

/// Integration test: runs all three strategies against the same store with user_id,
/// conversation_id and query set. Asserts each strategy returns the expected result type
/// and that combined context (recent messages + semantic hits + preferences) is consistent.
#[tokio::test]
async fn test_all_strategies_integration() {
    let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;

    let user_id = Some("user99".to_string());
    let conversation_id = Some("conv99".to_string());
    let query = Some("what did I say about food?".to_string());

    let metadata_conv = MemoryMetadata {
        user_id: Some("user99".to_string()),
        conversation_id: Some("conv99".to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };
    let metadata_pref = MemoryMetadata {
        user_id: Some("user99".to_string()),
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };

    store
        .add(MemoryEntry::new("Hello".to_string(), metadata_conv.clone()))
        .await
        .unwrap();
    store
        .add(MemoryEntry::new("I like pizza".to_string(), metadata_pref.clone()))
        .await
        .unwrap();
    store
        .add(MemoryEntry::new("I prefer tea".to_string(), metadata_pref))
        .await
        .unwrap();
    store
        .add(MemoryEntry::new("How are you?".to_string(), metadata_conv))
        .await
        .unwrap();

    let recent = RecentMessagesStrategy::new(10);
    let semantic = SemanticSearchStrategy::new(5, Arc::new(MockEmbeddingService));
    let preferences = UserPreferencesStrategy::new();

    let recent_result = recent
        .build_context(&*store, &user_id, &conversation_id, &query)
        .await
        .unwrap();
    let semantic_result = semantic
        .build_context(&*store, &user_id, &conversation_id, &query)
        .await
        .unwrap();
    let prefs_result = preferences
        .build_context(&*store, &user_id, &conversation_id, &query)
        .await
        .unwrap();

    match &recent_result {
        StrategyResult::Messages(msgs) => {
            assert_eq!(msgs.len(), 2, "recent messages should return conversation messages");
            assert!(msgs.join(" ").contains("Hello"));
            assert!(msgs.join(" ").contains("How are you?"));
        }
        _ => panic!("RecentMessagesStrategy should return Messages"),
    }

    match &semantic_result {
        StrategyResult::Messages(msgs) => {
            assert!(!msgs.is_empty(), "semantic search should return some messages");
        }
        StrategyResult::Empty => {}
        _ => panic!("SemanticSearchStrategy should return Messages or Empty"),
    }

    match &prefs_result {
        StrategyResult::Preferences(prefs) => {
            assert!(prefs.contains("User Preferences:"));
            assert!(prefs.contains("like") || prefs.contains("prefer"));
        }
        _ => panic!("UserPreferencesStrategy should return Preferences with seeded data"),
    }
}
