//! Context builder for assembling AI conversation context from strategies.

use super::types::{Context, ContextMetadata};
use super::utils::estimate_tokens;
use crate::memory::{MessageCategory, MemoryStore, StrategyResult};
use crate::memory::{ContextStrategy, StoreKind};
use std::sync::Arc;
use chrono::Utc;
use tracing::{debug, error, info, instrument};

/// Builder for constructing AI conversation context.
pub struct ContextBuilder {
    store: Arc<dyn MemoryStore>,
    pub(crate) recent_store: Option<Arc<dyn MemoryStore>>,
    pub(crate) strategies: Vec<Box<dyn ContextStrategy>>,
    pub(crate) token_limit: usize,
    pub(crate) user_id: Option<String>,
    pub(crate) conversation_id: Option<String>,
    pub(crate) query: Option<String>,
    pub(crate) system_message: Option<String>,
}

impl ContextBuilder {
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

    pub fn with_recent_store(mut self, recent_store: Arc<dyn MemoryStore>) -> Self {
        self.recent_store = Some(recent_store);
        self
    }

    pub fn with_strategy(mut self, strategy: Box<dyn ContextStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }

    pub fn with_token_limit(mut self, limit: usize) -> Self {
        self.token_limit = limit;
        self
    }

    pub fn for_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    pub fn for_conversation(mut self, conversation_id: &str) -> Self {
        self.conversation_id = Some(conversation_id.to_string());
        self
    }

    pub fn with_query(mut self, query: &str) -> Self {
        self.query = Some(query.to_string());
        self
    }

    pub fn with_system_message(mut self, message: &str) -> Self {
        self.system_message = Some(message.to_string());
        self
    }

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
