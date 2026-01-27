//! # Context Strategies
//!
//! This module provides strategies for building conversation context.
//!
//! Available strategies:
//! - `RecentMessagesStrategy`: Retrieves most recent messages
//! - `SemanticSearchStrategy`: Performs semantic search for relevant messages
//! - `UserPreferencesStrategy`: Extracts user preferences from history

use async_trait::async_trait;
use crate::context::StrategyResult;
use crate::store::MemoryStore;
use crate::types::{MemoryEntry, MemoryRole};

/// Trait for context building strategies.
#[async_trait]
pub trait ContextStrategy: Send + Sync {
    /// Builds context using strategy.
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        conversation_id: &Option<String>,
        query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error>;
}

/// Strategy for retrieving recent conversation messages.
#[derive(Debug, Clone)]
pub struct RecentMessagesStrategy {
    limit: usize,
}

impl RecentMessagesStrategy {
    /// Creates a new RecentMessagesStrategy.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of recent messages to retrieve.
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

#[async_trait]
impl ContextStrategy for RecentMessagesStrategy {
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        conversation_id: &Option<String>,
        _query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        // If conversation ID is provided, use it
        if let Some(conv_id) = conversation_id {
            let entries = store.search_by_conversation(conv_id).await?;

            let messages: Vec<String> = entries
                .into_iter()
                .take(self.limit)
                .map(|entry| format_message(&entry))
                .collect();

            return Ok(StrategyResult::Messages(messages));
        }

        // If only user ID is provided, search by user
        if let Some(uid) = user_id {
            let entries = store.search_by_user(uid).await?;

            let messages: Vec<String> = entries
                .into_iter()
                .take(self.limit)
                .map(|entry| format_message(&entry))
                .collect();

            return Ok(StrategyResult::Messages(messages));
        }

        Ok(StrategyResult::Empty)
    }
}

/// Strategy for performing semantic search on conversation history.
#[derive(Debug, Clone)]
pub struct SemanticSearchStrategy {
    limit: usize,
}

impl SemanticSearchStrategy {
    /// Creates a new SemanticSearchStrategy.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of relevant messages to retrieve.
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

#[async_trait]
impl ContextStrategy for SemanticSearchStrategy {
    async fn build_context(
        &self,
        _store: &dyn MemoryStore,
        _user_id: &Option<String>,
        _conversation_id: &Option<String>,
        query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        let query = match query {
            Some(q) => q,
            None => return Ok(StrategyResult::Empty),
        };

        // Note: This requires embedding service to be integrated
        // For now, we'll return empty result as semantic search
        // needs embedding generation for the query
        Ok(StrategyResult::Empty)
    }
}

/// Strategy for extracting user preferences from conversation history.
#[derive(Debug, Clone)]
pub struct UserPreferencesStrategy;

impl UserPreferencesStrategy {
    /// Creates a new UserPreferencesStrategy.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextStrategy for UserPreferencesStrategy {
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        _conversation_id: &Option<String>,
        _query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        let user_id = match user_id {
            Some(uid) => uid,
            None => return Ok(StrategyResult::Empty),
        };

        let entries = store.search_by_user(user_id).await?;

        let preferences = extract_preferences(&entries);

        if preferences.is_empty() {
            Ok(StrategyResult::Empty)
        } else {
            Ok(StrategyResult::Preferences(format!(
                "User Preferences: {}",
                preferences.join(", ")
            )))
        }
    }
}

/// Formats a memory entry as a message.
fn format_message(entry: &MemoryEntry) -> String {
    let role = match entry.metadata.role {
        MemoryRole::User => "User",
        MemoryRole::Assistant => "Assistant",
        MemoryRole::System => "System",
    };

    format!("{}: {}", role, entry.content)
}

/// Extracts preferences from conversation history.
fn extract_preferences(entries: &[MemoryEntry]) -> Vec<String> {
    let mut preferences = Vec::new();

    for entry in entries {
        let content = entry.content.to_lowercase();

        // Simple heuristic for detecting preferences
        if content.contains("i like") || content.contains("i prefer") {
            if let Some(start) = content.find("i like") {
                let preference = &content[start..];
                preferences.push(preference.to_string());
            } else if let Some(start) = content.find("i prefer") {
                let preference = &content[start..];
                preferences.push(preference.to_string());
            }
        }
    }

    preferences
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inmemory_store::InMemoryVectorStore;
    use crate::{MemoryEntry, MemoryMetadata, MemoryRole};
    use chrono::Utc;
    use std::sync::Arc;

    async fn create_test_store_with_entries() -> Arc<InMemoryVectorStore> {
        let store = Arc::new(InMemoryVectorStore::new());

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

        store
    }

    #[test]
    fn test_recent_messages_strategy_creation() {
        let strategy = RecentMessagesStrategy::new(5);
        assert_eq!(strategy.limit, 5);
    }

    #[tokio::test]
    async fn test_recent_messages_by_conversation() {
        let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>;
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
        let strategy = SemanticSearchStrategy::new(5);
        assert_eq!(strategy.limit, 5);
    }

    #[tokio::test]
    async fn test_semantic_search_no_query() {
        let store: Arc<dyn MemoryStore> = Arc::new(InMemoryVectorStore::new());
        let strategy = SemanticSearchStrategy::new(5);

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
        let store: Arc<dyn MemoryStore> = Arc::new(InMemoryVectorStore::new());

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

    #[test]
    fn test_format_message() {
        let metadata = MemoryMetadata {
            user_id: None,
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let entry = MemoryEntry::new("Hello world".to_string(), metadata);

        let formatted = format_message(&entry);
        assert_eq!(formatted, "User: Hello world");
    }
}
