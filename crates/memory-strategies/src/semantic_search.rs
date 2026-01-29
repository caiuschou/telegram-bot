//! Semantic search context strategy.
//!
//! Uses query embedding to find semantically relevant messages from the vector store.
//! External interactions: EmbeddingService; MemoryStore.semantic_search; AI context.

use std::sync::Arc;

use async_trait::async_trait;
use embedding::EmbeddingService;
use memory_core::{MemoryStore, StrategyResult};
use tracing::{debug, warn};

use super::strategy::ContextStrategy;
use super::utils::format_message;

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

        let query_embedding = match self.embedding_service.embed(query_text).await {
            Ok(emb) => emb,
            Err(e) => {
                warn!(error = %e, "SemanticSearchStrategy: embedding failed, skipping semantic search");
                return Ok(StrategyResult::Empty);
            }
        };

        let entries = store
            .semantic_search(&query_embedding, self.limit)
            .await?;

        let messages: Vec<String> = entries
            .into_iter()
            .map(|entry| format_message(&entry))
            .collect();

        debug!(
            message_count = messages.len(),
            "SemanticSearchStrategy: semantic search returned messages"
        );

        Ok(StrategyResult::Messages(messages))
    }
}
