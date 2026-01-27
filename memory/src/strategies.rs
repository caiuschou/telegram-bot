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
    /// Builds context by retrieving the most recent conversation messages.
    ///
    /// Prioritizes conversation-based retrieval if conversation_id is provided,
    /// otherwise falls back to user-based retrieval. Returns messages in
    /// chronological order as stored (typically already sorted by timestamp).
    ///
    /// # Priority Order
    ///
    /// 1. If conversation_id provided: search_by_conversation()
    /// 2. Else if user_id provided: search_by_user()
    /// 3. Else: return Empty result
    ///
    /// # External Interactions
    ///
    /// - **MemoryStore**: Queries database for conversation or user history
    /// - **Storage**: Retrieves persistent conversation data
    /// - **AI Context**: Results are formatted and included in AI conversation context
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
    /// Builds context by performing semantic search for relevant messages.
    ///
    /// Currently returns Empty result as semantic search requires integration
    /// with embedding service to generate embeddings for query text.
    ///
    /// # Planned Implementation
    ///
    /// 1. Generate embedding for query text using EmbeddingService
    /// 2. Call store.semantic_search() with query embedding
    /// 3. Format returned entries as messages
    ///
    /// # External Interactions
    ///
    /// - **Embedding Service**: Will call OpenAI API to generate query embedding
    /// - **MemoryStore**: Will perform vector similarity search
    /// - **AI Context**: Results provide semantically relevant context for current query
    ///
    /// # Current State
    ///
    /// Placeholder implementation - returns Empty result.
    /// TODO: Integrate with embedding service and complete implementation.
    async fn build_context(
        &self,
        _store: &dyn MemoryStore,
        _user_id: &Option<String>,
        _conversation_id: &Option<String>,
        _query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
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
    /// Builds context by extracting user preferences from conversation history.
    ///
    /// Analyzes all historical messages for a user to identify expressed
    /// preferences, which can be used to personalize AI responses.
    ///
    /// # Process
    ///
    /// 1. Retrieve all entries for the user from MemoryStore
    /// 2. Analyze messages for preference indicators ("I like", "I prefer")
    /// 3. Extract preference statements from matched messages
    /// 4. Return formatted preferences string or Empty if none found
    ///
    /// # External Interactions
    ///
    /// - **MemoryStore**: Queries all historical messages for the user
    /// - **Storage**: Reads persistent conversation history
    /// - **AI Personalization**: Extracted preferences enable personalized responses
    /// - **User Experience**: AI can recall and respect user preferences across sessions
    ///
    /// # Limitations
    ///
    /// - Uses simple pattern matching (not semantic analysis)
    /// - May miss preferences expressed in complex language
    /// - May include false positives in edge cases
    /// - Future: Use LLM to extract preferences more accurately
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

/// Formats a memory entry as a message string.
///
/// Converts a MemoryEntry into a human-readable message format suitable for
/// inclusion in AI conversation context. The format follows standard conversation
/// conventions with role prefixes.
///
/// # Format Pattern
///
/// "{Role}: {content}"
///
/// Where Role is one of: "User", "Assistant", "System"
///
/// # Example
///
/// ```text
/// User: Hello, how are you?
/// Assistant: I'm doing well, thank you!
/// System: You are a helpful assistant.
/// ```
///
/// # External Interactions
    ///
    /// - **AI Models**: Formatted messages are directly consumed by LLM APIs
    /// - **Conversation Parsers**: Follows standard role-based message format
    fn format_message(entry: &MemoryEntry) -> String {
    let role = match entry.metadata.role {
        MemoryRole::User => "User",
        MemoryRole::Assistant => "Assistant",
        MemoryRole::System => "System",
    };

    format!("{}: {}", role, entry.content)
}

/// Extracts user preferences from conversation history.
///
/// Analyzes historical messages to identify and extract user preferences expressed
/// during conversations. Uses simple pattern matching to detect preference statements.
///
    /// # Detection Patterns
    ///
    /// Scans for phrases indicating preferences:
    /// - "I like" followed by content
    /// - "I prefer" followed by content
    ///
    /// # Algorithm
    ///
    /// 1. Iterates through all provided memory entries
    /// 2. Converts content to lowercase for case-insensitive matching
    /// 3. Searches for preference indicator phrases
    /// 4. Extracts text from the first occurrence to end of message
    /// 5. Collects all unique preference statements found
    ///
    /// # Limitations
    ///
    /// - Simple keyword-based detection (may miss complex preference expressions)
    /// - Does not perform semantic analysis
    /// - May include false positives in edge cases
    /// - Future enhancement: Use NLP/LLM for more sophisticated extraction
    ///
    /// # External Interactions
    ///
    /// - **Memory Store**: Reads historical conversation data
    /// - **AI Context**: Extracted preferences are included in AI conversation context
    /// - **User Personalization**: Enables personalized AI responses based on preferences
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
