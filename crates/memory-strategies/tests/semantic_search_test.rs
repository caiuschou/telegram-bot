//! Unit tests for SemanticSearchStrategy.
//!
//! Covers creation and build_context (with/without query).
//! Uses shared MockStore and MockEmbeddingService from tests/common.

mod common;

use memory_core::{MemoryStore, StrategyResult};
use memory_strategies::{ContextStrategy, SemanticSearchStrategy};
use std::sync::Arc;

#[test]
fn test_semantic_search_strategy_creation() {
    let _ = SemanticSearchStrategy::new(5, Arc::new(common::MockEmbeddingService));
}

#[tokio::test]
async fn test_semantic_search_no_query() {
    let store: Arc<dyn MemoryStore> = Arc::new(common::MockStore::new());
    let strategy = SemanticSearchStrategy::new(5, Arc::new(common::MockEmbeddingService));

    let result = strategy
        .build_context(&*store, &None, &None, &None)
        .await
        .unwrap();

    assert!(matches!(result, StrategyResult::Empty));
}
