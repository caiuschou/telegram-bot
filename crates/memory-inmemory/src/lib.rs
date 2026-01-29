//! # In-Memory Vector Store
//!
//! This crate provides an in-memory implementation of the `MemoryStore` trait from the `memory` crate.
//!
//! ## InMemoryVectorStore
//!
//! Simple in-memory storage for testing and development.
//!
//! **Advantages**:
//! - Fastest performance (no I/O)
//! - Simple to set up and use
//! - Great for testing and prototyping
//!
//! **Limitations**:
//! - Data is lost on restart
//! - Not suitable for production use
//! - Limited by available memory
//!
//! ## Example
//!
//! ```rust
//! use memory_inmemory::InMemoryVectorStore;
//! use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
//! use chrono::Utc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), anyhow::Error> {
//!     let store = InMemoryVectorStore::new();
//!
//!     let metadata = MemoryMetadata {
//!         user_id: Some("user123".to_string()),
//!         conversation_id: None,
//!         role: MemoryRole::User,
//!         timestamp: Utc::now(),
//!         tokens: None,
//!         importance: None,
//!     };
//!     let entry = MemoryEntry::new("Hello world".to_string(), metadata);
//!
//!     store.add(entry).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Thread Safety
//!
//! The store uses `Arc<RwLock<>>` to ensure thread-safe concurrent access.

use memory::types::MemoryEntry;
use memory::store::MemoryStore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// In-memory vector store for testing and development.
#[derive(Debug, Clone)]
pub struct InMemoryVectorStore {
    entries: Arc<RwLock<HashMap<Uuid, MemoryEntry>>>,
}

impl InMemoryVectorStore {
    /// Creates a new empty in-memory vector store.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns the number of entries in the store.
    pub async fn len(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    /// Returns true if the store is empty.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Clears all entries from the store.
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Calculates cosine similarity between two vectors.
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
                "step: 词向量 InMemory 写入向量"
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

        let mut entries = self.entries.write().await;
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
        let entries = self.entries.read().await;
        let result = entries.get(&id).cloned();
        let found = result.is_some();
        info!(id = %id, found, "In-memory vector store get returned");
        Ok(result)
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
        info!(user_id = %user_id, "Querying in-memory vector store by user");
        let entries = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect();
        info!(
            user_id = %user_id,
            count = results.len(),
            "In-memory vector store search_by_user returned"
        );
        Ok(results)
    }

    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        info!(conversation_id = %conversation_id, "Querying in-memory vector store by conversation");
        let entries = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
            .cloned()
            .collect();
        info!(
            conversation_id = %conversation_id,
            count = results.len(),
            "In-memory vector store search_by_conversation returned"
        );
        Ok(results)
    }

    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        info!(
            dimension = query_embedding.len(),
            limit = limit,
            "step: 词向量 InMemory 向量检索"
        );
        info!(
            embedding_len = query_embedding.len(),
            limit = limit,
            "Querying in-memory vector store semantic_search"
        );
        let entries = self.entries.read().await;

        let mut similarities: Vec<(f32, MemoryEntry)> = entries
            .values()
            .filter_map(|entry| {
                entry.embedding.as_ref().map(|embedding| {
                    let similarity = Self::cosine_similarity(query_embedding, embedding);
                    (similarity, entry.clone())
                })
            })
            .collect();

        similarities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<MemoryEntry> = similarities
            .into_iter()
            .take(limit)
            .map(|(_, entry)| entry)
            .collect();

        info!(
            limit = limit,
            count = results.len(),
            "step: 词向量 InMemory 向量检索完成"
        );
        info!(
            limit = limit,
            count = results.len(),
            "In-memory vector store semantic_search returned"
        );
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory::{MemoryMetadata, MemoryRole};
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
    async fn test_update() {
        let store = InMemoryVectorStore::new();
        let mut entry = create_test_entry("Original", "user123");
        store.add(entry.clone()).await.unwrap();

        entry.content = "Updated".to_string();
        store.update(entry.clone()).await.unwrap();

        let found = store.get(entry.id).await.unwrap().unwrap();
        assert_eq!(found.content, "Updated");
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryVectorStore::new();
        let entry = create_test_entry("Test", "user123");
        store.add(entry.clone()).await.unwrap();

        store.delete(entry.id).await.unwrap();

        let found = store.get(entry.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_search_by_user() {
        let store = InMemoryVectorStore::new();

        let entry1 = create_test_entry("Hello", "user123");
        let entry2 = create_test_entry("World", "user123");
        let entry3 = create_test_entry("Other", "user456");

        store.add(entry1).await.unwrap();
        store.add(entry2).await.unwrap();
        store.add(entry3).await.unwrap();

        let results = store.search_by_user("user123").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_search_by_conversation() {
        let store = InMemoryVectorStore::new();

        let metadata1 = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: Some("conv1".to_string()),
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let entry1 = MemoryEntry::new("Hello".to_string(), metadata1.clone());

        let metadata2 = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: Some("conv2".to_string()),
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };
        let entry2 = MemoryEntry::new("World".to_string(), metadata2);

        store.add(entry1).await.unwrap();
        store.add(entry2).await.unwrap();

        let results = store.search_by_conversation("conv1").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_semantic_search() {
        let store = InMemoryVectorStore::new();

        let mut entry1 = create_test_entry("Hello world", "user123");
        entry1.embedding = Some(vec![1.0, 0.0, 0.0]);

        let mut entry2 = create_test_entry("Goodbye world", "user123");
        entry2.embedding = Some(vec![0.0, 1.0, 0.0]);

        let mut entry3 = create_test_entry("Hello there", "user123");
        entry3.embedding = Some(vec![0.9, 0.1, 0.0]);

        store.add(entry1).await.unwrap();
        store.add(entry2).await.unwrap();
        store.add(entry3).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.semantic_search(&query, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].content, "Hello world");
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

    #[tokio::test]
    async fn test_clear() {
        let store = InMemoryVectorStore::new();

        let entry = create_test_entry("Test", "user123");
        store.add(entry).await.unwrap();

        assert_eq!(store.len().await, 1);

        store.clear().await;

        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        // Identical vectors
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!((InMemoryVectorStore::cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        // Orthogonal vectors
        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!(InMemoryVectorStore::cosine_similarity(&c, &d).abs() < 1e-6);

        // Empty vectors
        let e: Vec<f32> = vec![];
        assert_eq!(InMemoryVectorStore::cosine_similarity(&e, &a), 0.0);
    }
}
