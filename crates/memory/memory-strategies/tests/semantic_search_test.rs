//! Unit tests for SemanticSearchStrategy.
//!
//! Covers creation, build_context (with/without query), and min_score threshold filtering.
//! Uses shared MockStore and MockEmbeddingService from tests/common.

mod common;

use memory_core::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, StrategyResult};
use memory_strategies::{ContextStrategy, SemanticSearchStrategy};
use std::sync::Arc;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn test_semantic_search_strategy_creation() {
    let _ = SemanticSearchStrategy::new(5, Arc::new(common::MockEmbeddingService), 0.0);
}

#[tokio::test]
async fn test_semantic_search_no_query() {
    let store: Arc<dyn MemoryStore> = Arc::new(common::MockStore::new());
    let strategy = SemanticSearchStrategy::new(5, Arc::new(common::MockEmbeddingService), 0.0);

    let result = strategy
        .build_context(&*store, &None, &None, &None)
        .await
        .unwrap();

    assert!(matches!(result, StrategyResult::Empty));
}

/// Store that returns fixed (score, entry) pairs for semantic_search. Used to test threshold filtering.
struct ScoredMockStore {
    scored_entries: Vec<(f32, MemoryEntry)>,
}

impl ScoredMockStore {
    fn new(scored_entries: Vec<(f32, MemoryEntry)>) -> Self {
        Self { scored_entries }
    }
}

#[async_trait::async_trait]
impl MemoryStore for ScoredMockStore {
    async fn add(&self, _entry: MemoryEntry) -> Result<(), anyhow::Error> {
        Ok(())
    }
    async fn get(&self, _id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        Ok(None)
    }
    async fn update(&self, _entry: MemoryEntry) -> Result<(), anyhow::Error> {
        Ok(())
    }
    async fn delete(&self, _id: Uuid) -> Result<(), anyhow::Error> {
        Ok(())
    }
    async fn search_by_user(&self, _user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        Ok(Vec::new())
    }
    async fn search_by_conversation(
        &self,
        _conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        Ok(Vec::new())
    }
    async fn semantic_search(
        &self,
        _query_embedding: &[f32],
        limit: usize,
        _user_id: Option<&str>,
        _conversation_id: Option<&str>,
    ) -> Result<Vec<(f32, MemoryEntry)>, anyhow::Error> {
        let mut out: Vec<_> = self.scored_entries.iter().cloned().collect();
        out.truncate(limit);
        Ok(out)
    }
}

#[tokio::test]
async fn test_semantic_search_min_score_filters_low_scores() {
    let meta = MemoryMetadata {
        user_id: None,
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };
    let e_low = MemoryEntry {
        id: Uuid::new_v4(),
        content: "low score".to_string(),
        embedding: None,
        metadata: meta.clone(),
    };
    let e_high = MemoryEntry {
        id: Uuid::new_v4(),
        content: "high score".to_string(),
        embedding: None,
        metadata: meta,
    };
    let store = Arc::new(ScoredMockStore::new(vec![(0.4, e_low), (0.9, e_high)]));
    let strategy = SemanticSearchStrategy::new(5, Arc::new(common::MockEmbeddingService), 0.7);

    let result = strategy
        .build_context(
            &*store,
            &None,
            &None,
            &Some("query".to_string()),
        )
        .await
        .unwrap();

    let messages = match &result {
        StrategyResult::Messages { messages: m, .. } => m,
        _ => panic!("expected Messages"),
    };
    assert_eq!(messages.len(), 1, "only entry with score >= 0.7 should remain");
    assert!(messages[0].contains("high score"));
}

#[tokio::test]
async fn test_semantic_search_min_score_zero_keeps_all() {
    let meta = MemoryMetadata {
        user_id: None,
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };
    let e1 = MemoryEntry {
        id: Uuid::new_v4(),
        content: "first".to_string(),
        embedding: None,
        metadata: meta.clone(),
    };
    let e2 = MemoryEntry {
        id: Uuid::new_v4(),
        content: "second".to_string(),
        embedding: None,
        metadata: meta,
    };
    let store = Arc::new(ScoredMockStore::new(vec![(0.3, e1), (0.6, e2)]));
    let strategy = SemanticSearchStrategy::new(5, Arc::new(common::MockEmbeddingService), 0.0);

    let result = strategy
        .build_context(
            &*store,
            &None,
            &None,
            &Some("query".to_string()),
        )
        .await
        .unwrap();

    let messages = match &result {
        StrategyResult::Messages { messages: m, .. } => m,
        _ => panic!("expected Messages"),
    };
    assert_eq!(messages.len(), 2, "min_score 0.0 should keep all entries");
}
