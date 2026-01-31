//! Semantic search context strategy.

use std::sync::Arc;
use async_trait::async_trait;
use crate::embedding::EmbeddingService;
use crate::memory_core::{MessageCategory, MemoryEntry, MemoryStore, StrategyResult};
use tracing::{debug, error, info, warn};

use super::strategy::ContextStrategy;
use super::utils::format_message;

pub struct SemanticSearchStrategy {
    limit: usize,
    min_score: f32,
    embedding_service: Arc<dyn EmbeddingService>,
}

impl SemanticSearchStrategy {
    pub fn new(limit: usize, embedding_service: Arc<dyn EmbeddingService>, min_score: f32) -> Self {
        Self { limit, min_score, embedding_service }
    }
}

#[async_trait]
impl ContextStrategy for SemanticSearchStrategy {
    fn name(&self) -> &str {
        "SemanticSearch"
    }
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        _user_id: &Option<String>,
        conversation_id: &Option<String>,
        query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        let query_text = match query {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => {
                debug!("SemanticSearchStrategy: no query text, skipping semantic search");
                return Ok(StrategyResult::Empty);
            }
        };
        info!(query_len = query_text.len(), limit = self.limit, "SemanticSearchStrategy: starting semantic search");
        info!(query = %query_text, query_len = query_text.len(), "step: embedding generate query vector");
        let query_embedding = match self.embedding_service.embed(query_text).await {
            Ok(emb) => { info!(dimension = emb.len(), "step: embedding query vector done"); emb }
            Err(e) => {
                warn!(error = %e, query = %query_text, "SemanticSearchStrategy: embedding failed, skipping semantic search");
                return Ok(StrategyResult::Empty);
            }
        };
        info!(dimension = query_embedding.len(), limit = self.limit, min_score = self.min_score, "step: embedding semantic_search");
        let scored_entries = match store.semantic_search(&query_embedding, self.limit, None, conversation_id.as_deref()).await {
            Ok(ent) => ent,
            Err(e) => {
                error!(error = %e, query = %query_text, limit = self.limit, "SemanticSearchStrategy: semantic_search failed");
                return Err(anyhow::anyhow!("SemanticSearchStrategy semantic_search failed: {}", e));
            }
        };
        let count_before = scored_entries.len();
        if count_before > 0 {
            let scores: Vec<f32> = scored_entries.iter().map(|(s, _)| *s).collect();
            let min_s = scores.iter().cloned().fold(f32::NAN, f32::min);
            let max_s = scores.iter().cloned().fold(f32::NAN, f32::max);
            let mean_s = scores.iter().sum::<f32>() / scores.len() as f32;
            info!(count = count_before, score_min = %min_s, score_mean = %mean_s, score_max = %max_s, "step: embedding semantic_search score distribution");
        }
        let entries: Vec<MemoryEntry> = scored_entries
            .into_iter()
            .filter(|(score, _)| *score >= self.min_score)
            .map(|(_, entry)| entry)
            .collect();
        if count_before > 0 && entries.is_empty() {
            warn!(query = %query_text, min_score = self.min_score, count_before = count_before, "SemanticSearchStrategy: all semantic results below threshold");
        }
        let messages: Vec<String> = entries.iter().map(|entry| format_message(entry)).collect();
        info!(entry_count = entries.len(), "step: embedding semantic_search done");
        info!(query = %query_text, entry_count = entries.len(), message_count = messages.len(), "SemanticSearchStrategy: semantic search returned messages");
        Ok(StrategyResult::Messages { category: MessageCategory::Semantic, messages })
    }
}
