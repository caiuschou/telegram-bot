//! Recent messages context strategy.
//!
//! Retrieves the most recent conversation or user messages for context.
//! External interactions: MemoryStore (search_by_conversation, search_by_user); AI context.

use async_trait::async_trait;
use memory_core::{MemoryStore, StrategyResult};
use tracing::debug;

use super::strategy::ContextStrategy;
use super::utils::format_message;

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
        if let Some(conv_id) = conversation_id {
            debug!(
                conversation_id = conv_id,
                limit = self.limit,
                "RecentMessagesStrategy: searching by conversation_id"
            );
            let entries = store.search_by_conversation(conv_id).await?;

            let messages: Vec<String> = entries
                .into_iter()
                .take(self.limit)
                .map(|entry| format_message(&entry))
                .collect();

            debug!(
                message_count = messages.len(),
                "RecentMessagesStrategy: found messages by conversation_id"
            );
            return Ok(StrategyResult::Messages(messages));
        }

        if let Some(uid) = user_id {
            debug!(
                user_id = uid,
                limit = self.limit,
                "RecentMessagesStrategy: searching by user_id"
            );
            let entries = store.search_by_user(uid).await?;

            let messages: Vec<String> = entries
                .into_iter()
                .take(self.limit)
                .map(|entry| format_message(&entry))
                .collect();

            debug!(
                message_count = messages.len(),
                "RecentMessagesStrategy: found messages by user_id"
            );
            return Ok(StrategyResult::Messages(messages));
        }

        debug!("RecentMessagesStrategy: no user_id or conversation_id, returning Empty");
        Ok(StrategyResult::Empty)
    }
}
