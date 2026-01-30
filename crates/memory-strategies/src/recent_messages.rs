//! Recent messages context strategy.
//!
//! Retrieves the most recent conversation or user messages for context.
//! External interactions: MemoryStore (search_by_conversation, search_by_user); AI context.

use async_trait::async_trait;
use memory_core::{MessageCategory, MemoryStore, StrategyResult};
use tracing::{debug, info};

use super::strategy::{ContextStrategy, StoreKind};
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
    fn name(&self) -> &str {
        "RecentMessages"
    }

    fn store_kind(&self) -> StoreKind {
        StoreKind::Recent
    }

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
            let mut entries = store.search_by_conversation(conv_id).await.map_err(|e| {
                tracing::error!(
                    error = %e,
                    conversation_id = %conv_id,
                    "RecentMessagesStrategy: search_by_conversation failed"
                );
                e
            })?;
            // 排除 content 为空的条目，避免上下文出现 "User: " / "Assistant: " 无内容行（可能来自历史错误写入或列序问题）
            entries.retain(|e| !e.content.is_empty());
            entries.sort_by_key(|e| e.metadata.timestamp);
            let n = entries.len();
            if n > self.limit {
                entries = entries.into_iter().skip(n - self.limit).collect();
            }

            let messages: Vec<String> = entries
                .iter()
                .map(|entry| format_message(entry))
                .collect();

            info!(
                conversation_id = %conv_id,
                entry_count = entries.len(),
                message_count = messages.len(),
                "RecentMessagesStrategy: 最近消息 by conversation_id"
            );
            for (i, msg) in messages.iter().enumerate() {
                info!(strategy = "RecentMessagesStrategy", index = i, content = %msg, "最近消息");
            }
            return Ok(StrategyResult::Messages {
                category: MessageCategory::Recent,
                messages,
            });
        }

        if let Some(uid) = user_id {
            debug!(
                user_id = uid,
                limit = self.limit,
                "RecentMessagesStrategy: searching by user_id"
            );
            let mut entries = store.search_by_user(uid).await.map_err(|e| {
                tracing::error!(
                    error = %e,
                    user_id = %uid,
                    "RecentMessagesStrategy: search_by_user failed"
                );
                e
            })?;
            entries.retain(|e| !e.content.is_empty());
            entries.sort_by_key(|e| e.metadata.timestamp);
            let n = entries.len();
            if n > self.limit {
                entries = entries.into_iter().skip(n - self.limit).collect();
            }

            let messages: Vec<String> = entries
                .iter()
                .map(|entry| format_message(entry))
                .collect();

            info!(
                user_id = %uid,
                entry_count = entries.len(),
                message_count = messages.len(),
                "RecentMessagesStrategy: 最近消息 by user_id"
            );
            for (i, msg) in messages.iter().enumerate() {
                info!(strategy = "RecentMessagesStrategy", index = i, content = %msg, "最近消息");
            }
            return Ok(StrategyResult::Messages {
                category: MessageCategory::Recent,
                messages,
            });
        }

        debug!("RecentMessagesStrategy: no user_id or conversation_id, returning Empty");
        Ok(StrategyResult::Empty)
    }
}
