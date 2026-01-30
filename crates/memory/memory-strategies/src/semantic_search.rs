//! Semantic search context strategy.
//!
//! Uses query embedding to find semantically relevant messages from the vector store.
//! External interactions: EmbeddingService; MemoryStore.semantic_search; AI context.

use std::sync::Arc;

use async_trait::async_trait;
use embedding::EmbeddingService;
use memory_core::{MessageCategory, MemoryEntry, MemoryStore, StrategyResult};
use tracing::{debug, error, info, warn};

use super::strategy::ContextStrategy;
use super::utils::format_message;

/// Strategy for performing semantic search on conversation history.
///
/// Uses the user's question text to generate an embedding, then searches the vector store
/// for the most semantically similar memory entries to include as context.
/// Entries with score < min_score are filtered out (see docs/rag/memory/vector-search-accuracy.md).
pub struct SemanticSearchStrategy {
    limit: usize,
    /// Minimum similarity score (e.g. cosine). Entries with score < min_score are excluded. 0.0 = no filter.
    min_score: f32,
    embedding_service: Arc<dyn EmbeddingService>,
}

impl SemanticSearchStrategy {
    /// Creates a new SemanticSearchStrategy.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of relevant messages to retrieve.
    /// * `embedding_service` - Service to generate query embedding (e.g. OpenAI).
    /// * `min_score` - Minimum similarity score; entries below this are filtered. Use 0.0 to disable (default behavior).
    pub fn new(
        limit: usize,
        embedding_service: Arc<dyn EmbeddingService>,
        min_score: f32,
    ) -> Self {
        Self {
            limit,
            min_score,
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
        user_id: &Option<String>,
        conversation_id: &Option<String>,
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

        info!(query = %query_text, query_len = query_text.len(), "step: embedding generate query vector");
        let query_embedding = match self.embedding_service.embed(query_text).await {
            Ok(emb) => {
                info!(
                    dimension = emb.len(),
                    "step: embedding query vector done"
                );
                emb
            }
            Err(e) => {
                warn!(error = %e, query = %query_text, "SemanticSearchStrategy: embedding failed, skipping semantic search");
                return Ok(StrategyResult::Empty);
            }
        };

        info!(
            dimension = query_embedding.len(),
            limit = self.limit,
            min_score = self.min_score,
            "step: embedding semantic_search"
        );
        let scored_entries = match store
            .semantic_search(
                &query_embedding,
                self.limit,
                user_id.as_deref(),
                conversation_id.as_deref(),
            )
            .await
        {
            Ok(ent) => ent,
            Err(e) => {
                let err_msg = format!("{:?}", e);
                error!(
                    error = %e,
                    error_debug = %err_msg,
                    query = %query_text,
                    limit = self.limit,
                    "SemanticSearchStrategy: semantic_search failed"
                );
                return Err(anyhow::anyhow!(
                    "SemanticSearchStrategy semantic_search failed: {}",
                    e
                ));
            }
        };

        // Observability: log score distribution (min/mean/max) for top_k before threshold filter
        let count_before = scored_entries.len();
        if count_before > 0 {
            let scores: Vec<f32> = scored_entries.iter().map(|(s, _)| *s).collect();
            let min_s = scores.iter().cloned().fold(f32::NAN, f32::min);
            let max_s = scores.iter().cloned().fold(f32::NAN, f32::max);
            let mean_s = scores.iter().sum::<f32>() / scores.len() as f32;
            info!(
                count = count_before,
                score_min = %min_s,
                score_mean = %mean_s,
                score_max = %max_s,
                "step: embedding semantic_search score distribution"
            );
        }

        let entries: Vec<MemoryEntry> = scored_entries
            .into_iter()
            .filter(|(score, _)| *score >= self.min_score)
            .map(|(_, entry)| entry)
            .collect();

        if count_before > 0 && entries.is_empty() {
            warn!(
                query = %query_text,
                min_score = self.min_score,
                count_before = count_before,
                "SemanticSearchStrategy: all semantic results below threshold, no entries kept"
            );
        }

        let messages: Vec<String> = entries
            .iter()
            .map(|entry| format_message(entry))
            .collect();

        info!(
            entry_count = entries.len(),
            "step: embedding semantic_search done"
        );
        info!(
            query = %query_text,
            entry_count = entries.len(),
            message_count = messages.len(),
            "SemanticSearchStrategy: semantic search returned messages"
        );
        for (i, msg) in messages.iter().enumerate() {
            info!(strategy = "SemanticSearchStrategy", index = i, content = %msg, "semantic search");
        }

        Ok(StrategyResult::Messages {
            category: MessageCategory::Semantic,
            messages,
        })
    }
}
