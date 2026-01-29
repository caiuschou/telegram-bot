//! Unit tests for UserPreferencesStrategy.
//!
//! Covers creation and preference extraction from user history.
//! Uses shared MockStore from tests/common.

mod common;

use chrono::Utc;
use memory_core::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, StrategyResult};
use memory_strategies::{ContextStrategy, UserPreferencesStrategy};
use std::sync::Arc;

#[test]
fn test_user_preferences_strategy_creation() {
    let _ = UserPreferencesStrategy::new();
}

#[tokio::test]
async fn test_user_preferences_extraction() {
    let store: Arc<dyn MemoryStore> = Arc::new(common::MockStore::new());

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
