//! Shared test utilities for memory-strategies integration tests.
//!
//! Provides MockStore (MemoryStore) and MockEmbeddingService (EmbeddingService)
//! used by strategy test files under tests/.

use async_trait::async_trait;
use embedding::EmbeddingService;
use memory_core::{MemoryEntry, MemoryStore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Mock embedding service: returns a fixed-dimension vector for any input.
/// Used by SemanticSearchStrategy tests without calling external embedding APIs.
#[allow(dead_code)]
pub struct MockEmbeddingService;

#[async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, anyhow::Error> {
        Ok(vec![0.0; 1536])
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        Ok(texts.iter().map(|_| vec![0.0; 1536]).collect())
    }
}

/// In-memory store implementing MemoryStore for tests.
/// Filters by user_id / conversation_id and supports semantic_search (returns entries up to limit).
pub struct MockStore {
    entries: Arc<RwLock<HashMap<Uuid, MemoryEntry>>>,
}

impl MockStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MemoryStore for MockStore {
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries.get(&id).cloned())
    }

    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.remove(&id);
        Ok(())
    }

    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect())
    }

    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
            .cloned()
            .collect())
    }

    async fn semantic_search(
        &self,
        _query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries.values().take(limit).cloned().collect())
    }
}
