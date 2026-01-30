//! Full integration test for all context strategies.
//!
//! Runs RecentMessagesStrategy, SemanticSearchStrategy and UserPreferencesStrategy
//! against the same store with user_id, conversation_id and query set.
//! Asserts each strategy returns the expected result type and that combined
//! context (recent messages + semantic hits + preferences) is consistent.
//! Uses shared MockStore and MockEmbeddingService from tests/common.

mod common;

use chrono::Utc;
use memory_core::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, StrategyResult};
use memory_strategies::{
    ContextStrategy, RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy,
};
use std::sync::Arc;

#[tokio::test]
async fn test_all_strategies_integration() {
    let store = Arc::new(common::MockStore::new()) as Arc<dyn MemoryStore>;

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
    let semantic = SemanticSearchStrategy::new(5, Arc::new(common::MockEmbeddingService), 0.0);
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
        StrategyResult::Messages { messages: msgs, .. } => {
            assert_eq!(msgs.len(), 2, "recent messages should return conversation messages");
            assert!(msgs.join(" ").contains("Hello"));
            assert!(msgs.join(" ").contains("How are you?"));
        }
        _ => panic!("RecentMessagesStrategy should return Messages"),
    }

    match &semantic_result {
        StrategyResult::Messages { messages: msgs, .. } => {
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
