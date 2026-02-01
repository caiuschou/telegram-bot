//! In-memory implementation of the MemoryStore trait.

use super::{MemoryEntry, MemoryStore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

type EntryMap = HashMap<Uuid, MemoryEntry>;

/// In-memory vector store for testing and development.
#[derive(Debug, Clone)]
pub struct InMemoryVectorStore {
    entries: Arc<RwLock<EntryMap>>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(EntryMap::new())),
        }
    }

    pub async fn len(&self) -> usize {
        let entries: tokio::sync::RwLockReadGuard<'_, EntryMap> = self.entries.read().await;
        entries.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    pub async fn clear(&self) {
        let mut entries: tokio::sync::RwLockWriteGuard<'_, EntryMap> = self.entries.write().await;
        entries.clear();
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        dot_product / (norm_a * norm_b)
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MemoryStore for InMemoryVectorStore {
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        if entry.embedding.is_some() {
            info!(
                id = %entry.id,
                dimension = entry.embedding.as_ref().map(|e| e.len()).unwrap_or(0),
                "step: embedding InMemory write vector"
            );
        }
        info!(
            id = %entry.id,
            user_id = ?entry.metadata.user_id,
            conversation_id = ?entry.metadata.conversation_id,
            role = ?entry.metadata.role,
            has_embedding = entry.embedding.is_some(),
            "Writing entry to in-memory vector store"
        );
        let mut entries: tokio::sync::RwLockWriteGuard<'_, EntryMap> = self.entries.write().await;
        entries.insert(entry.id, entry.clone());
        drop(entries);
        info!(
            id = %entry.id,
            user_id = ?entry.metadata.user_id,
            conversation_id = ?entry.metadata.conversation_id,
            "Entry written to in-memory vector store"
        );
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        info!(id = %id, "Querying in-memory vector store by id");
        let entries: tokio::sync::RwLockReadGuard<'_, EntryMap> = self.entries.read().await;
        let result = entries.get(&id).cloned();
        let found = result.is_some();
        info!(id = %id, found, "In-memory vector store get returned");
        Ok(result)
    }

    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries: tokio::sync::RwLockWriteGuard<'_, EntryMap> = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error> {
        let mut entries: tokio::sync::RwLockWriteGuard<'_, EntryMap> = self.entries.write().await;
        entries.remove(&id);
        Ok(())
    }

    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        info!(user_id = %user_id, "Querying in-memory vector store by user");
        let entries: tokio::sync::RwLockReadGuard<'_, EntryMap> = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect();
        info!(user_id = %user_id, count = results.len(), "In-memory vector store search_by_user returned");
        Ok(results)
    }

    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        info!(conversation_id = %conversation_id, "Querying in-memory vector store by conversation");
        let entries: tokio::sync::RwLockReadGuard<'_, EntryMap> = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
            .cloned()
            .collect();
        info!(conversation_id = %conversation_id, count = results.len(), "In-memory vector store search_by_conversation returned");
        Ok(results)
    }

    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        user_id: Option<&str>,
        conversation_id: Option<&str>,
    ) -> Result<Vec<(f32, MemoryEntry)>, anyhow::Error> {
        info!(dimension = query_embedding.len(), limit = limit, user_id = ?user_id, conversation_id = ?conversation_id, "step: embedding InMemory semantic search");
        let entries: tokio::sync::RwLockReadGuard<'_, EntryMap> = self.entries.read().await;
        let mut similarities: Vec<(f32, MemoryEntry)> = entries
            .values()
            .filter(|entry| {
                let match_user = user_id
                    .map(|u| entry.metadata.user_id.as_deref() == Some(u))
                    .unwrap_or(true);
                let match_conv = conversation_id
                    .map(|c| entry.metadata.conversation_id.as_deref() == Some(c))
                    .unwrap_or(true);
                match_user && match_conv
            })
            .filter_map(|entry| {
                entry.embedding.as_ref().map(|embedding| {
                    let similarity = Self::cosine_similarity(query_embedding, embedding);
                    (similarity, entry.clone())
                })
            })
            .collect();
        similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let results: Vec<(f32, MemoryEntry)> = similarities.into_iter().take(limit).collect();
        info!(limit = limit, count = results.len(), "step: embedding InMemory semantic search done");
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryMetadata, MemoryRole};
    use chrono::Utc;

    fn create_test_entry(content: &str, user_id: &str) -> MemoryEntry {
        let metadata = MemoryMetadata {
            user_id: Some(user_id.to_string()),
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        MemoryEntry::new(content.to_string(), metadata)
    }

    #[tokio::test]
    async fn test_add_and_get() {
        let store = InMemoryVectorStore::new();
        let entry = create_test_entry("Test", "user123");
        store.add(entry.clone()).await.unwrap();
        let found = store.get(entry.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Test");
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = InMemoryVectorStore::new();
        let found = store.get(Uuid::new_v4()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_len_and_is_empty() {
        let store = InMemoryVectorStore::new();
        assert!(store.is_empty().await);
        assert_eq!(store.len().await, 0);
        let entry = create_test_entry("Test", "user123");
        store.add(entry).await.unwrap();
        assert!(!store.is_empty().await);
        assert_eq!(store.len().await, 1);
    }
}
