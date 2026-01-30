//! Context builder for assembling AI conversation context from strategies.
//!
//! Orchestrates MemoryStore and context strategies to produce a final Context.
//! External: MemoryStore, embedding services (via strategies), AI models.

use super::types::{Context, ContextMetadata};
use super::utils::estimate_tokens;
use memory_core::{MessageCategory, MemoryStore, StrategyResult};
use memory_strategies::{ContextStrategy, StoreKind};
use std::sync::Arc;
use chrono::Utc;
use tracing::{debug, error, info, instrument};

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
///
/// # Dual store (recent vs semantic)
///
/// When `recent_store` is set, strategies with `StoreKind::Recent` (RecentMessages, UserPreferences)
/// use it; others (e.g. SemanticSearch) use the primary `store`. This allows recent messages from
/// SQLite while semantic search uses e.g. Lance.
pub struct ContextBuilder {
    store: Arc<dyn MemoryStore>,
    /// When set, RecentMessagesStrategy and UserPreferencesStrategy use this store; SemanticSearchStrategy uses `store`.
    pub(crate) recent_store: Option<Arc<dyn MemoryStore>>,
    /// Exposed for tests that assert builder configuration.
    pub(crate) strategies: Vec<Box<dyn ContextStrategy>>,
    /// Exposed for tests that assert builder configuration.
    pub(crate) token_limit: usize,
    /// Exposed for tests that assert builder configuration.
    pub(crate) user_id: Option<String>,
    /// Exposed for tests that assert builder configuration.
    pub(crate) conversation_id: Option<String>,
    /// Exposed for tests that assert builder configuration.
    pub(crate) query: Option<String>,
    /// Exposed for tests that assert builder configuration.
    pub(crate) system_message: Option<String>,
}

impl ContextBuilder {
    /// Creates a new ContextBuilder with given memory store.
    pub fn new(store: Arc<dyn MemoryStore>) -> Self {
        Self {
            store,
            recent_store: None,
            strategies: Vec::new(),
            token_limit: 4096,
            user_id: None,
            conversation_id: None,
            query: None,
            system_message: None,
        }
    }

    /// Uses the given store for RecentMessagesStrategy and UserPreferencesStrategy; semantic search still uses the primary store.
    pub fn with_recent_store(mut self, recent_store: Arc<dyn MemoryStore>) -> Self {
        self.recent_store = Some(recent_store);
        self
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

        for (strategy_index, strategy) in self.strategies.iter().enumerate() {
            let strategy_name = strategy.name();
            let store: &dyn MemoryStore = match strategy.store_kind() {
                StoreKind::Recent if self.recent_store.is_some() => {
                    self.recent_store.as_deref().unwrap()
                }
                _ => self.store.as_ref(),
            };
            info!(strategy_index, strategy_name, "Executing context strategy");
            let result = strategy
                .build_context(store, &self.user_id, &self.conversation_id, &self.query)
                .await
                .map_err(|e| {
                    error!(strategy_index, strategy_name, error = %e, "Context build: strategy failed");
                    for (i, cause) in e.chain().enumerate() {
                        if i > 0 {
                            error!(cause = %cause, "Caused by");
                        }
                    }
                    e
                })?;
            apply_strategy_result(
                strategy_name,
                strategy_index,
                result,
                &mut recent_messages,
                &mut semantic_messages,
                &mut preferences,
            );
        }

        let message_count = recent_messages.len() + semantic_messages.len();
        let total_tokens =
            self.calculate_total_tokens(&recent_messages, &semantic_messages, &preferences);

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

        log_context_detail(&recent_messages, &semantic_messages, &preferences);

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
    /// the configured token limit. Uses a simple approximation (1 token ≈ 4 characters).
    fn calculate_total_tokens(
        &self,
        recent_messages: &[String],
        semantic_messages: &[String],
        preferences: &Option<String>,
    ) -> usize {
        self.system_message
            .iter()
            .chain(recent_messages.iter())
            .chain(semantic_messages.iter())
            .chain(preferences.iter())
            .map(|s| estimate_tokens(s))
            .sum()
    }
}

/// Logs context components in detail for debugging: recent messages, semantic search, user preferences.
fn log_context_detail(
    recent_messages: &[String],
    semantic_messages: &[String],
    preferences: &Option<String>,
) {
    info!(
        count = recent_messages.len(),
        "context_detail: recent messages"
    );
    for (i, msg) in recent_messages.iter().enumerate() {
        info!(index = i, content = %msg, "recent messages");
    }
    info!(
        count = semantic_messages.len(),
        "context_detail: semantic search"
    );
    for (i, msg) in semantic_messages.iter().enumerate() {
        info!(index = i, content = %msg, "semantic search");
    }
    if let Some(prefs) = preferences {
        info!(preferences = %prefs, "context_detail: user preferences");
    } else {
        info!("context_detail: user preferences (none)");
    }
}

/// Applies a strategy result to the context accumulators and logs it.
fn apply_strategy_result(
    strategy_name: &str,
    strategy_index: usize,
    result: StrategyResult,
    recent_messages: &mut Vec<String>,
    semantic_messages: &mut Vec<String>,
    preferences: &mut Option<String>,
) {
    match result {
        StrategyResult::Messages { category, messages } => {
            let total_len: usize = messages.iter().map(|m| m.len()).sum();
            let label = match category {
                MessageCategory::Recent => "recent messages",
                MessageCategory::Semantic => "semantic search",
            };
            info!(
                strategy_name,
                strategy_index,
                message_count = messages.len(),
                total_content_len = total_len,
                label,
                "Strategy returned messages"
            );
            for (i, msg) in messages.iter().enumerate() {
                info!(strategy_name, index = i, content = %msg, label, "strategy message");
            }
            match category {
                MessageCategory::Recent => recent_messages.extend(messages),
                MessageCategory::Semantic => semantic_messages.extend(messages),
            }
        }
        StrategyResult::Preferences(prefs) => {
            info!(
                strategy_name,
                strategy_index,
                preferences = %prefs,
                "Strategy returned user preferences"
            );
            *preferences = Some(prefs);
        }
        StrategyResult::Empty => {
            info!(strategy_name, strategy_index, "Strategy returned Empty");
        }
    }
}
