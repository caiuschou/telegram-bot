//! Unit tests for RecentMessagesStrategy.
//!
//! Covers creation and build_context by conversation_id / user_id.
//! Uses shared MockStore from tests/common.

mod common;

use chrono::Utc;
use memory_core::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, StrategyResult};
use memory_strategies::{ContextStrategy, RecentMessagesStrategy};
use std::sync::Arc;

#[test]
fn test_recent_messages_strategy_creation() {
    let _ = RecentMessagesStrategy::new(5);
}

#[tokio::test]
async fn test_recent_messages_by_conversation() {
    let store = Arc::new(common::MockStore::new()) as Arc<dyn MemoryStore>;
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
        StrategyResult::Messages { messages: msgs, .. } => {
            assert_eq!(msgs.len(), 2);
            let combined = msgs.join(" ");
            assert!(combined.contains("How are you?"));
            assert!(combined.contains("Hello"));
        }
        _ => panic!("Expected Messages result"),
    }
}
