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
//! use memory::RecentMessagesStrategy;
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

use memory_core::{MessageCategory, MemoryStore};
use memory_strategies::ContextStrategy;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tracing::{debug, error, info, instrument};

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
/// - recent_messages: Main dialogue record (recent conversation) for the AI
/// - semantic_messages: Retrieved reference context from semantic search
/// - user_preferences: Extracted user preferences for personalization
/// - metadata: Context metadata including token counts and timestamps
#[derive(Debug, Clone)]
pub struct Context {
    /// System message if provided
    pub system_message: Option<String>,
    /// Recent conversation messages — main dialogue record for the AI.
    pub recent_messages: Vec<String>,
    /// Semantically retrieved messages — reference context for the current query.
    pub semantic_messages: Vec<String>,
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
/// - Messages with category Recent go to recent_messages (main dialogue)
/// - Messages with category Semantic go to semantic_messages (reference)
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

        let mut recent_messages = Vec::new();
        let mut semantic_messages = Vec::new();
        let mut preferences: Option<String> = None;

        // Execute strategies in order (RecentMessages, SemanticSearch, UserPreferences)
        for (strategy_index, strategy) in self.strategies.iter().enumerate() {
            let strategy_name = strategy.name();
            info!(
                strategy_index,
                strategy_name,
                "Executing context strategy"
            );
            let strategy_result = strategy
                .build_context(
                    &*self.store,
                    &self.user_id,
                    &self.conversation_id,
                    &self.query,
                )
                .await
                .map_err(|e| {
                    error!(
                        strategy_index,
                        strategy_name,
                        error = %e,
                        "Context build: strategy failed"
                    );
                    for (i, cause) in e.chain().enumerate() {
                        if i > 0 {
                            error!(cause = %cause, "Caused by");
                        }
                    }
                    e
                })?;

            match strategy_result {
                memory_core::StrategyResult::Messages { category, messages } => {
                    let total_len: usize = messages.iter().map(|m| m.len()).sum();
                    info!(
                        strategy_name,
                        strategy_index,
                        message_count = messages.len(),
                        total_content_len = total_len,
                        "Strategy returned messages"
                    );
                    for (i, msg) in messages.iter().enumerate() {
                        let preview = truncate_for_log(msg, 400);
                        debug!(
                            strategy_name,
                            strategy_index,
                            message_index = i,
                            message_len = msg.len(),
                            content_preview = %preview,
                            "Context message from strategy"
                        );
                    }
                    match category {
                        MessageCategory::Recent => recent_messages.extend(messages),
                        MessageCategory::Semantic => semantic_messages.extend(messages),
                    }
                }
                memory_core::StrategyResult::Preferences(prefs) => {
                    info!(
                        strategy_name,
                        strategy_index,
                        preferences_preview = %truncate_for_log(&prefs, 400),
                        "Strategy returned user preferences"
                    );
                    preferences = Some(prefs);
                }
                memory_core::StrategyResult::Empty => {
                    info!(
                        strategy_name,
                        strategy_index,
                        "Strategy returned Empty"
                    );
                }
            }
        }

        let message_count = recent_messages.len() + semantic_messages.len();
        // Calculate tokens
        let total_tokens =
            self.calculate_total_tokens(&recent_messages, &semantic_messages, &preferences);

        // Create metadata
        let metadata = ContextMetadata {
            user_id: self.user_id.clone(),
            conversation_id: self.conversation_id.clone(),
            total_tokens,
            message_count,
            created_at: Utc::now(),
        };

        debug!(
            total_tokens = metadata.total_tokens,
            message_count = metadata.message_count,
            "Finished context build"
        );

        Ok(Context {
            system_message: self.system_message.clone(),
            recent_messages,
            semantic_messages,
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
        recent_messages: &[String],
        semantic_messages: &[String],
        preferences: &Option<String>,
    ) -> usize {
        let mut total = 0;

        // System message tokens
        if let Some(ref msg) = self.system_message {
            total += estimate_tokens(msg);
        }

        // Recent and semantic message tokens
        for msg in recent_messages.iter().chain(semantic_messages.iter()) {
            total += estimate_tokens(msg);
        }

        // Preferences tokens
        if let Some(ref prefs) = preferences {
            total += estimate_tokens(prefs);
        }

        total
    }
}

/// Truncates a string for logging; appends "..." if truncated.
/// Used when logging strategy message content to avoid dumping huge strings.
fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
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
    /// Returns context as a single string for AI models (no current question).
    ///
    /// Delegates to `prompt::format_for_model`. Used when only the context block is needed
    /// (e.g. middleware returning context string). External: output sent to LLM APIs.
    pub fn format_for_model(&self, include_system: bool) -> String {
        prompt::format_for_model(
            include_system,
            self.system_message.as_deref(),
            self.user_preferences.as_deref(),
            &self.recent_messages,
            &self.semantic_messages,
        )
    }

    /// Returns context as chat messages with different types (system, user, assistant).
    ///
    /// Calls `prompt::format_for_model_as_messages_with_roles` so recent conversation
    /// lines ("User: ...", "Assistant: ...", "System: ...") become separate `ChatMessage`
    /// with matching roles. Order: optional System, parsed recent (User/Assistant/System),
    /// optional User(preferences+semantic block), User(question).
    pub fn to_messages(&self, include_system: bool, current_question: &str) -> Vec<prompt::ChatMessage> {
        prompt::format_for_model_as_messages_with_roles(
            include_system,
            self.system_message.as_deref(),
            self.user_preferences.as_deref(),
            &self.recent_messages,
            &self.semantic_messages,
            current_question,
        )
    }

    /// Returns true if there are no recent and no semantic messages.
    pub fn is_empty(&self) -> bool {
        self.recent_messages.is_empty() && self.semantic_messages.is_empty()
    }

    /// Checks if the context exceeds the token limit.
    pub fn exceeds_limit(&self, limit: usize) -> bool {
        self.metadata.total_tokens > limit
    }
}

#[cfg(test)]
mod estimate_tokens_test;

#[cfg(test)]
mod context_builder_test;

#[cfg(test)]
mod context_test;
