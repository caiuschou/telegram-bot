//! Semantic search context strategy.
//!
//! Uses query embedding to find semantically relevant messages from the vector store.
//! External interactions: EmbeddingService; MemoryStore.semantic_search; AI context.

use std::sync::Arc;

use async_trait::async_trait;
use embedding::EmbeddingService;
use memory_core::{MessageCategory, MemoryStore, StrategyResult};
use tracing::{debug, error, info, warn};

use super::strategy::ContextStrategy;
use super::utils::{format_message, truncate_for_log, MAX_LOG_CONTENT_LEN};

/// Strategy for performing semantic search on conversation history.
///
/// Uses the user's question text to generate an embedding, then searches the vector store
/// for the most semantically similar memory entries to include as context.
pub struct SemanticSearchStrategy {
    limit: usize,
    embedding_service: Arc<dyn EmbeddingService>,
}

impl SemanticSearchStrategy {
    /// Creates a new SemanticSearchStrategy.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of relevant messages to retrieve.
    /// * `embedding_service` - Service to generate query embedding (e.g. OpenAI).
    pub fn new(limit: usize, embedding_service: Arc<dyn EmbeddingService>) -> Self {
        Self {
            limit,
            embedding_service,
        }
    }
}

#[async_trait]
impl ContextStrategy for SemanticSearchStrategy {
    fn name(&self) -> &str {
        "SemanticSearch"
    }

    /// Builds context by performing semantic search for relevant messages.
    ///
    /// 1. If query text is present, generates embedding via EmbeddingService.
    /// 2. Calls store.semantic_search() with query embedding.
    /// 3. Formats returned entries as messages and returns them.
    ///
    /// # External Interactions
    ///
    /// - **Embedding Service**: Calls embedding API to generate query vector.
    /// - **MemoryStore**: Performs vector similarity search (e.g. Lance/SQLite).
    /// - **AI Context**: Results provide semantically relevant context for the current query.
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        _user_id: &Option<String>,
        _conversation_id: &Option<String>,
        query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        let query_text = match query {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => {
                debug!(
                    "SemanticSearchStrategy: no query text, skipping semantic search"
                );
                return Ok(StrategyResult::Empty);
            }
        };

        info!(
            query_len = query_text.len(),
            limit = self.limit,
            "SemanticSearchStrategy: starting semantic search"
        );

        info!(
            query_preview = %truncate_for_log(query_text, MAX_LOG_CONTENT_LEN),
            query_len = query_text.len(),
            "step: 词向量 生成查询向量 (embedding)"
        );
        let query_embedding = match self.embedding_service.embed(query_text).await {
            Ok(emb) => {
                info!(
                    dimension = emb.len(),
                    "step: 词向量 查询向量生成完成"
                );
                emb
            }
            Err(e) => {
                warn!(
                    error = %e,
                    query_preview = %truncate_for_log(query_text, MAX_LOG_CONTENT_LEN),
                    "SemanticSearchStrategy: embedding failed, skipping semantic search"
                );
                return Ok(StrategyResult::Empty);
            }
        };

        info!(
            dimension = query_embedding.len(),
            limit = self.limit,
            "step: 词向量 向量检索 (semantic_search)"
        );
        let entries = match store
            .semantic_search(&query_embedding, self.limit)
            .await
        {
            Ok(ent) => ent,
            Err(e) => {
                let err_msg = format!("{:?}", e);
                error!(
                    error = %e,
                    error_debug = %err_msg,
                    query_preview = %truncate_for_log(query_text, MAX_LOG_CONTENT_LEN),
                    limit = self.limit,
                    "SemanticSearchStrategy: semantic_search failed"
                );
                return Err(anyhow::anyhow!(
                    "SemanticSearchStrategy semantic_search failed: {}",
                    e
                ));
            }
        };

        let messages: Vec<String> = entries
            .iter()
            .map(|entry| format_message(entry))
            .collect();

        info!(
            entry_count = entries.len(),
            "step: 词向量 向量检索完成"
        );
        info!(
            query_preview = %truncate_for_log(query_text, MAX_LOG_CONTENT_LEN),
            entry_count = entries.len(),
            message_count = messages.len(),
            "SemanticSearchStrategy: semantic search returned messages"
        );
        for (i, msg) in messages.iter().enumerate() {
            info!(
                strategy = "SemanticSearchStrategy",
                index = i,
                content_preview = %truncate_for_log(msg, MAX_LOG_CONTENT_LEN),
                "context message"
            );
        }

        Ok(StrategyResult::Messages {
            category: MessageCategory::Semantic,
            messages,
        })
    }
}
