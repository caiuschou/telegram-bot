//! # Context Builder
//!
//! This module provides the `ContextBuilder` for constructing AI conversation context
//! from memory store using various strategies.
//!
//! ## ContextBuilder
//!
//! Main builder class that orchestrates context construction using different strategies.
//!
//! ## Example
//!
//! ```rust
//! use memory::context::ContextBuilder;
//! use memory::strategies::RecentMessagesStrategy;
//! use memory_inmemory::InMemoryVectorStore;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), anyhow::Error> {
//! let store = Arc::new(InMemoryVectorStore::new());
//! let builder = ContextBuilder::new(store)
//!     .with_token_limit(4096);
//!
//! let context = builder
//!     .for_user("user123")
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::store::MemoryStore;
use crate::strategies::ContextStrategy;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tracing::{debug, instrument};

/// Represents a constructed context for AI conversation.
///
/// Contains all the information needed to provide context to an AI model,
/// including system instructions, conversation history, and user preferences.
///
/// # External Interactions
///
/// - **AI Models**: Formatted context is sent to LLM APIs (OpenAI, Anthropic, etc.)
/// - **Memory Management**: Context size must fit within model's token limit
/// - **Conversation State**: Maintains continuity across multi-turn conversations
///
/// # Components
///
/// - system_message: Optional AI personality/behavior instructions
/// - conversation_history: Chronological sequence of messages
/// - user_preferences: Extracted user preferences for personalization
/// - metadata: Context metadata including token counts and timestamps
#[derive(Debug, Clone)]
pub struct Context {
    /// System message if provided
    pub system_message: Option<String>,
    /// Conversation history formatted for AI input
    pub conversation_history: Vec<String>,
    /// User preferences extracted from history
    pub user_preferences: Option<String>,
    /// Metadata about the context
    pub metadata: ContextMetadata,
}

/// Metadata about the constructed context.
///
/// Provides diagnostic information about the context, useful for monitoring,
/// debugging, and ensuring context stays within token limits.
///
/// # External Interactions
///
/// - **Monitoring**: Metadata can be logged for observability
/// - **Token Management**: total_tokens helps prevent exceeding API limits
/// - **Analytics**: message_count and timestamps enable usage analysis
#[derive(Debug, Clone)]
pub struct ContextMetadata {
    /// User ID for this context
    pub user_id: Option<String>,
    /// Conversation ID for this context
    pub conversation_id: Option<String>,
    /// Total estimated token count
    pub total_tokens: usize,
    /// Number of messages in context
    pub message_count: usize,
    /// When the context was built
    pub created_at: DateTime<Utc>,
}

/// Builder for constructing AI conversation context.
///
/// Orchestrates the assembly of conversation context by applying multiple
/// context strategies in sequence. Each strategy can contribute different
/// types of information to the final context (messages, preferences, etc.).
///
/// # External Interactions
///
/// - **MemoryStore**: Delegates data retrieval to configured strategies
/// - **Embedding Services**: Strategies may use embedding services for semantic search
/// - **AI Models**: Constructed context is formatted for LLM API consumption
///
/// # Strategy Execution Order
///
/// Strategies are executed in the order they were added. Each strategy's
/// result is aggregated into the final context:
/// - Messages are appended to conversation_history
/// - Preferences replace previous preferences (last strategy wins)
/// - Empty results are ignored
pub struct ContextBuilder {
    store: Arc<dyn MemoryStore>,
    strategies: Vec<Box<dyn ContextStrategy>>,
    token_limit: usize,
    user_id: Option<String>,
    conversation_id: Option<String>,
    query: Option<String>,
    system_message: Option<String>,
}

impl ContextBuilder {
    /// Creates a new ContextBuilder with given memory store.
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self {
            store,
            strategies: Vec::new(),
            token_limit: 4096,
            user_id: None,
            conversation_id: None,
            query: None,
            system_message: None,
        }
    }

    /// Adds a context strategy to the builder.
    pub fn with_strategy(mut self, strategy: Box<dyn ContextStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }

    /// Sets the maximum token limit for the context.
    pub fn with_token_limit(mut self, limit: usize) -> Self {
        self.token_limit = limit;
        self
    }

    /// Sets the user ID for this context.
    pub fn for_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// Sets the conversation ID for this context.
    pub fn for_conversation(mut self, conversation_id: &str) -> Self {
        self.conversation_id = Some(conversation_id.to_string());
        self
    }

    /// Sets the query for semantic search strategies.
    pub fn with_query(mut self, query: &str) -> Self {
        self.query = Some(query.to_string());
        self
    }

    /// Sets a custom system message.
    pub fn with_system_message(mut self, message: &str) -> Self {
        self.system_message = Some(message.to_string());
        self
    }

    /// Builds the context using all configured strategies.
    ///
    /// This method executes each registered strategy in sequence, collecting their results
    /// and combining them into a final Context. It interacts with the MemoryStore to retrieve
    /// conversation data and orchestrates the context construction process.
    ///
    /// # Process
    ///
    /// 1. Iterates through all registered strategies in order
    /// 2. For each strategy, calls `build_context()` which may:
    ///    - Query the MemoryStore for conversation history
    ///    - Perform semantic searches (if embedding data available)
    ///    - Extract user preferences from historical messages
    /// 3. Aggregates results from all strategies
    /// 4. Calculates total token count for the assembled context
    /// 5. Constructs and returns the final Context object
    ///
    /// # External Interactions
    ///
    /// - **MemoryStore**: Queries for conversation data based on user_id and conversation_id
    /// - **Embedding Service**: Indirectly used through semantic search strategies
    ///
    /// # Returns
    ///
    /// A constructed `Context` containing system message, conversation history,
    /// user preferences, and metadata.
    #[instrument(
        skip(self),
        fields(
            user_id = ?self.user_id,
            conversation_id = ?self.conversation_id,
            strategy_count = self.strategies.len()
        )
    )]
    pub async fn build(&self) -> Result<Context, anyhow::Error> {
        debug!("Starting context build");

        let mut history = Vec::new();
        let mut preferences: Option<String> = None;

        // Execute strategies in order
        for strategy in &self.strategies {
            debug!("Executing context strategy");
            let strategy_result = strategy
                .build_context(
                    &*self.store,
                    &self.user_id,
                    &self.conversation_id,
                    &self.query,
                )
                .await?;

            match strategy_result {
                StrategyResult::Messages(messages) => {
                    debug!(message_count = messages.len(), "Strategy returned messages");
                    history.extend(messages);
                }
                StrategyResult::Preferences(prefs) => {
                    debug!("Strategy returned user preferences");
                    preferences = Some(prefs);
                }
                StrategyResult::Empty => {}
            }
        }

        // Calculate tokens
        let total_tokens = self.calculate_total_tokens(&history, &preferences);

        // Create metadata
        let metadata = ContextMetadata {
            user_id: self.user_id.clone(),
            conversation_id: self.conversation_id.clone(),
            total_tokens,
            message_count: history.len(),
            created_at: Utc::now(),
        };

        debug!(
            total_tokens = metadata.total_tokens,
            message_count = metadata.message_count,
            "Finished context build"
        );

        Ok(Context {
            system_message: self.system_message.clone(),
            conversation_history: history,
            user_preferences: preferences,
            metadata,
        })
    }

    /// Calculates the total token count for context.
    ///
    /// Estimates token usage for all context components to ensure it stays within
    /// the configured token limit. This prevents exceeding API token limits when
    /// sending context to AI models.
    ///
    /// # Calculation Components
    ///
    /// - System message tokens (if provided)
    /// - Conversation history tokens (all messages)
    /// - User preferences tokens (if available)
    ///
    /// # External Interactions
    ///
    /// - **AI Models**: Token estimation ensures compatibility with model context windows
    ///
    /// # Note
    ///
    /// Uses a simple approximation (1 token ≈ 4 characters). For production use with
    /// precise token limits, consider using tiktoken library for accurate estimation.
    fn calculate_total_tokens(
        &self,
        history: &[String],
        preferences: &Option<String>,
    ) -> usize {
        let mut total = 0;

        // System message tokens
        if let Some(ref msg) = self.system_message {
            total += estimate_tokens(msg);
        }

        // History tokens
        for msg in history {
            total += estimate_tokens(msg);
        }

        // Preferences tokens
        if let Some(ref prefs) = preferences {
            total += estimate_tokens(prefs);
        }

        total
    }
}

/// Estimates the token count for a text string.
///
/// This is a rough approximation: 1 token ≈ 4 characters for English text.
/// For production use, consider using tiktoken for more accurate estimation.
///
/// # Algorithm
///
/// Divides text length by 4 and rounds up, ensuring minimum of 1 token.
///
/// # External Interactions
///
/// - **AI Models**: Token count determines if context fits within model's context window
/// - **Cost Calculation**: Token usage directly impacts API costs for LLM providers
pub fn estimate_tokens(text: &str) -> usize {
    ((text.len() as f64) / 4.0).ceil().max(1.0) as usize
}

impl Context {
    /// Formats the context for AI model input.
    ///
    /// Constructs a formatted string representation of the context suitable for
    /// passing to AI language models. The format is designed to be easily parsed
    /// by models and follows common conversation formatting patterns.
    ///
    /// # Format Structure
    ///
    /// ```text
    /// System: {system_message}
    ///
    /// User Preferences: {preferences}
    ///
    /// {message_1}
    /// {message_2}
    /// ...
    /// ```
    ///
    /// # Arguments
    ///
    /// * `include_system` - If true, includes the system message in the output
    ///
    /// # Returns
    ///
    /// A newline-separated string ready for submission to AI models.
    ///
    /// # External Interactions
    ///
    /// - **AI Models**: Formatted string is directly consumed by LLM APIs
    /// - **Conversation Parsers**: Format follows standard conversation patterns
    pub fn format_for_model(&self, include_system: bool) -> String {
        let mut output = String::new();

        // Add system message if requested
        if include_system {
            if let Some(ref system_msg) = self.system_message {
                output.push_str(&format!("System: {}\n\n", system_msg));
            }
        }

        // Add user preferences if available
        if let Some(ref prefs) = self.user_preferences {
            output.push_str(&format!("User Preferences: {}\n\n", prefs));
        }

        // Add conversation history
        for msg in &self.conversation_history {
            output.push_str(msg);
            output.push('\n');
        }

        output
    }

    /// Checks if the context exceeds the token limit.
    pub fn exceeds_limit(&self, limit: usize) -> bool {
        self.metadata.total_tokens > limit
    }
}

/// Result type for context strategies.
pub enum StrategyResult {
    Messages(Vec<String>),
    Preferences(String),
    Empty,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryEntry, MemoryMetadata, MemoryRole};
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
            Ok(entries.values()
                .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
                .cloned()
                .collect())
        }

        async fn search_by_conversation(
            &self,
            conversation_id: &str,
        ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
            let entries = self.entries.read().await;
            Ok(entries.values()
                .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
                .cloned()
                .collect())
        }

        async fn semantic_search(
            &self,
            _query_embedding: &[f32],
            _limit: usize,
        ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
            Ok(Vec::new())
        }
    }

    fn create_test_entry(content: &str, user_id: &str) -> MemoryEntry {
        let metadata = MemoryMetadata {
            user_id: Some(user_id.to_string()),
            conversation_id: Some("conv1".to_string()),
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        MemoryEntry::new(content.to_string(), metadata)
    }

    struct MockStrategy;

    #[async_trait::async_trait]
    impl ContextStrategy for MockStrategy {
        async fn build_context(
            &self,
            _store: &dyn MemoryStore,
            _user_id: &Option<String>,
            _conversation_id: &Option<String>,
            _query: &Option<String>,
        ) -> Result<super::StrategyResult, anyhow::Error> {
            Ok(super::StrategyResult::Messages(vec![
                "User: Hello".to_string(),
                "Assistant: Hi there!".to_string(),
            ]))
        }
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("Hello"), 2);
        assert_eq!(estimate_tokens("Hello world"), 3);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens(""), 1);
    }

    #[tokio::test]
    async fn test_context_builder_creation() {
        let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
        let builder = ContextBuilder::new(store)
            .with_token_limit(2048);

        assert_eq!(builder.token_limit, 2048);
    }

    #[tokio::test]
    async fn test_context_builder_with_user() {
        let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
        let builder = ContextBuilder::new(store)
            .for_user("user123");

        assert_eq!(builder.user_id.as_deref(), Some("user123"));
    }

    #[tokio::test]
    async fn test_context_builder_with_strategies() {
        let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
        let strategy = Box::new(MockStrategy);

        let builder = ContextBuilder::new(store)
            .with_strategy(strategy);

        assert_eq!(builder.strategies.len(), 1);
    }

    #[tokio::test]
    async fn test_context_builder_with_system_message() {
        let store = Arc::new(MockStore::new()) as Arc<dyn MemoryStore>;
        let builder = ContextBuilder::new(store)
            .with_system_message("You are a helpful assistant.");

        assert_eq!(
            builder.system_message.as_deref(),
            Some("You are a helpful assistant.")
        );
    }
}
