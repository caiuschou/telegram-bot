//! # In-Memory Vector Store
//!
//! This module provides an in-memory implementation of the `MemoryStore` trait.
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
//! use memory::inmemory_store::InMemoryVectorStore;
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

use crate::types::MemoryEntry;
use crate::store::MemoryStore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
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
    ///
    /// Provides count of all memory entries currently stored in memory.
    ///
    /// # External Interactions
    ///
    /// - **Memory Management**: Useful for monitoring in-memory usage
    /// - **Testing**: Helps verify test setup and cleanup
    /// - **Monitoring**: Can be logged for system observability
    pub async fn len(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    /// Returns true if the store is empty.
    ///
    /// Checks whether any memory entries are currently stored.
    ///
    /// # External Interactions
    ///
    /// - **Testing**: Useful for assertions in test cases
    /// - **Monitoring**: Can indicate if any conversation data exists
    /// - **Initialization**: Helps verify clean state
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Clears all entries from the store.
    ///
    /// Removes all memory entries from the in-memory store. This operation
    /// is instantaneous and frees memory for garbage collection.
    ///
    /// # External Interactions
    ///
    /// - **Memory Management**: Frees memory allocated for all entries
    /// - **Testing**: Enables clean test isolation
    /// - **State Reset**: Allows resetting to empty state
    ///
    /// # Warning
    ///
    /// This operation is irreversible and all data is permanently lost.
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }

    /// Calculates cosine similarity between two vectors.
    ///
    /// Computes the cosine similarity metric, which measures the cosine of the angle
    /// between two vectors. This is a standard similarity metric for vector embeddings,
    /// ranging from -1 (opposite) to 1 (identical), with 0 indicating orthogonality.
    ///
    /// # Algorithm
    ///
    /// Similarity = (a · b) / (||a|| * ||b||)
    ///
    /// Where:
    /// - a · b = dot product (sum of element-wise products)
    /// - ||a|| = Euclidean norm (square root of sum of squares)
    ///
    /// # Special Cases
    ///
    /// - Empty vectors return 0.0 similarity
    /// - Zero vectors return 0.0 similarity (to avoid division by zero)
    ///
    /// # External Interactions
    ///
    /// - **Semantic Search**: Used to rank memory entries by relevance to query
    /// - **Vector Databases**: Standard similarity metric for embedding comparisons
    ///
    /// # Performance
    ///
    /// Time complexity: O(n) where n is vector dimensionality.
    /// Memory complexity: O(1) - only accumulators used.
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
    /// Adds a new memory entry to the store.
    ///
    /// Stores a memory entry in the in-memory HashMap, making it available
    /// for future queries. The entry is indexed by its UUID for O(1) lookup.
    ///
    /// # External Interactions
    ///
    /// - **Memory Management**: Allocates memory in RAM (no disk I/O)
    /// - **Concurrent Access**: Uses RwLock for thread-safe access
    /// - **No Persistence**: Data is lost on application restart
    ///
    /// # Performance
    ///
    /// - Time complexity: O(1) average case for HashMap insertion
    /// - Memory overhead: Size of entry plus HashMap bucket overhead
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }

    /// Retrieves a memory entry by its UUID. Returns `None` if not found.
    ///
    /// Performs a direct lookup by UUID using the HashMap index.
    ///
    /// # External Interactions
    ///
    /// - **Memory Access**: Direct RAM lookup (no disk I/O)
    /// - **Concurrent Access**: Uses RwLock read lock for thread safety
    ///
    /// # Performance
    ///
    /// - Time complexity: O(1) average case for HashMap lookup
    /// - Returns cloned entry to maintain store ownership
    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries.get(&id).cloned())
    }

    /// Updates an existing memory entry.
    ///
    /// Replaces the existing entry with the same UUID. If no entry with
    /// the UUID exists, this operation adds it as a new entry.
    ///
    /// # External Interactions
    ///
    /// - **Memory Management**: Updates in RAM (no disk I/O)
    /// - **Concurrent Access**: Uses RwLock write lock for thread safety
    ///
    /// # Performance
    ///
    /// - Time complexity: O(1) average case for HashMap insertion
    /// - Replaces existing entry completely (no partial updates)
    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }

    /// Deletes a memory entry by its UUID.
    ///
    /// Removes a memory entry from the HashMap. If no entry with the UUID
    /// exists, this operation succeeds silently.
    ///
    /// # External Interactions
    ///
    /// - **Memory Management**: Frees memory for deleted entry (after GC)
    /// - **Concurrent Access**: Uses RwLock write lock for thread safety
    ///
    /// # Performance
    ///
    /// - Time complexity: O(1) average case for HashMap deletion
    /// - Memory is reclaimed by Rust's garbage collector
    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.remove(&id);
        Ok(())
    }

    /// Retrieves all memory entries for a specific user.
    ///
    /// Iterates through all entries and filters by user_id. This is a
    /// linear scan operation (no indexing for user_id).
    ///
    /// # External Interactions
    ///
    /// - **Memory Access**: Scans all entries in RAM
    /// - **Concurrent Access**: Uses RwLock read lock for thread safety
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is total number of entries
    /// - Space complexity: O(k) where k is number of matching entries
    /// - Consider adding user_id index for improved performance
    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect();
        Ok(results)
    }

    /// Retrieves all memory entries for a specific conversation.
    ///
    /// Iterates through all entries and filters by conversation_id. This is
    /// a linear scan operation (no indexing for conversation_id).
    ///
    /// # External Interactions
    ///
    /// - **Memory Access**: Scans all entries in RAM
    /// - **Concurrent Access**: Uses RwLock read lock for thread safety
    ///
    /// # Performance
    ///
    /// - Time complexity: O(n) where n is total number of entries
    /// - Space complexity: O(k) where k is number of matching entries
    /// - Consider adding conversation_id index for improved performance
    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
            .cloned()
            .collect();
        Ok(results)
    }

    /// Performs semantic search using vector embeddings.
    ///
    /// Returns the top `limit` most similar entries based on cosine similarity.
    /// This method finds memory entries that are semantically similar to the query
    /// by comparing their vector embeddings.
    ///
    /// # Algorithm
    ///
    /// 1. Iterates through all entries in memory
    /// 2. Filters entries that have embeddings (skips those without)
    /// 3. Calculates cosine similarity between query and each entry's embedding
    /// 4. Sorts entries by similarity score in descending order
    /// 5. Returns top `limit` entries with highest similarity
    ///
    /// # External Interactions
    ///
    /// - **Embedding Services**: Query embedding typically comes from OpenAI embedding API
    /// - **Memory Operations**: All data is already in memory (in-memory store)
    ///
    /// # Performance Characteristics
    ///
    /// - Time complexity: O(n * d) where n is number of entries, d is vector dimension
    /// - Memory complexity: O(1) additional (data already loaded)
    /// - Fastest semantic search among all store implementations
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - Vector embedding of the search query
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Vector of memory entries sorted by similarity (highest first).
    /// Entries without embeddings are excluded from results.
    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
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

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryMetadata, MemoryRole};
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
