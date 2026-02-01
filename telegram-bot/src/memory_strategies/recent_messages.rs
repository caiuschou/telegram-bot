//! Recent messages context strategy.

use async_trait::async_trait;
use crate::memory_core::{MessageCategory, MemoryStore, StrategyResult};
use tracing::{debug, info};

use super::strategy::{ContextStrategy, StoreKind};
use super::utils::format_message;

#[derive(Debug, Clone)]
pub struct RecentMessagesStrategy {
    limit: usize,
}

impl RecentMessagesStrategy {
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
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        conversation_id: &Option<String>,
        _query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        if let Some(conv_id) = conversation_id {
            debug!(conversation_id = conv_id, limit = self.limit, "RecentMessagesStrategy: searching by conversation_id");
            let mut entries = store.search_by_conversation(conv_id).await.map_err(|e| {
                tracing::error!(error = %e, conversation_id = %conv_id, "RecentMessagesStrategy: search_by_conversation failed");
                e
            })?;
            entries.retain(|e| !e.content.is_empty());
            entries.sort_by_key(|e| e.metadata.timestamp);
            let n = entries.len();
            if n > self.limit {
                entries = entries.into_iter().skip(n - self.limit).collect();
            }
            let messages: Vec<String> = entries.iter().map(|entry| format_message(entry)).collect();
            info!(conversation_id = %conv_id, entry_count = entries.len(), message_count = messages.len(), "RecentMessagesStrategy: recent messages by conversation_id");
            return Ok(StrategyResult::Messages { category: MessageCategory::Recent, messages });
        }
        if let Some(uid) = user_id {
            debug!(user_id = uid, limit = self.limit, "RecentMessagesStrategy: searching by user_id");
            let mut entries = store.search_by_user(uid).await.map_err(|e| {
                tracing::error!(error = %e, user_id = %uid, "RecentMessagesStrategy: search_by_user failed");
                e
            })?;
            entries.retain(|e| !e.content.is_empty());
            entries.sort_by_key(|e| e.metadata.timestamp);
            let n = entries.len();
            if n > self.limit {
                entries = entries.into_iter().skip(n - self.limit).collect();
            }
            let messages: Vec<String> = entries.iter().map(|entry| format_message(entry)).collect();
            info!(user_id = %uid, entry_count = entries.len(), message_count = messages.len(), "RecentMessagesStrategy: recent messages by user_id");
            return Ok(StrategyResult::Messages { category: MessageCategory::Recent, messages });
        }
        debug!("RecentMessagesStrategy: no user_id or conversation_id, returning Empty");
        Ok(StrategyResult::Empty)
    }
}
