//! Mock MemoryStore implementation for tests.
//!
//! In-memory [`MemoryStore`] using `HashMap<Uuid, MemoryEntry>`. Supports search_by_user and search_by_conversation.
//! `semantic_search` returns entries (optionally limited); call counters (store_call_count, query_call_count, semantic_search_call_count) allow assertions.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use uuid::Uuid;

/// In-memory MemoryStore for tests. Counters (store_call_count, query_call_count, semantic_search_call_count) for assertions.
#[derive(Debug, Clone)]
pub struct MockMemoryStore {
    inner: Arc<Mutex<HashMap<Uuid, MemoryEntry>>>,
    store_call_count: Arc<AtomicUsize>,
    query_call_count: Arc<AtomicUsize>,
    /// Incremented only on semantic_search; used to assert embed + vector search ran.
    semantic_search_call_count: Arc<AtomicUsize>,
}

impl Default for MockMemoryStore {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            store_call_count: Arc::new(AtomicUsize::new(0)),
            query_call_count: Arc::new(AtomicUsize::new(0)),
            semantic_search_call_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl MockMemoryStore {
    /// Creates a new empty MockMemoryStore.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of add/update calls (user messages and AI replies written).
    pub fn get_store_call_count(&self) -> usize {
        self.store_call_count.load(Ordering::SeqCst)
    }

    /// Returns the number of query calls (search_by_user, search_by_conversation, or semantic_search).
    pub fn get_query_call_count(&self) -> usize {
        self.query_call_count.load(Ordering::SeqCst)
    }

    /// Returns the number of semantic_search calls (incremented only when semantic_search is invoked).
    pub fn get_semantic_search_call_count(&self) -> usize {
        self.semantic_search_call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl MemoryStore for MockMemoryStore {
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        self.store_call_count.fetch_add(1, Ordering::SeqCst);
        let mut map = self.inner.lock().unwrap();
        map.insert(entry.id, entry);
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        let map = self.inner.lock().unwrap();
        Ok(map.get(&id).cloned())
    }

    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut map = self.inner.lock().unwrap();
        map.insert(entry.id, entry);
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error> {
        let mut map = self.inner.lock().unwrap();
        map.remove(&id);
        Ok(())
    }

    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        self.query_call_count.fetch_add(1, Ordering::SeqCst);
        let map = self.inner.lock().unwrap();
        Ok(map
            .values()
            .cloned()
            .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
            .collect())
    }

    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        self.query_call_count.fetch_add(1, Ordering::SeqCst);
        let map = self.inner.lock().unwrap();
        Ok(map
            .values()
            .cloned()
            .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
            .collect())
    }

    async fn semantic_search(
        &self,
        _query_embedding: &[f32],
        limit: usize,
        user_id: Option<&str>,
        conversation_id: Option<&str>,
    ) -> Result<Vec<(f32, MemoryEntry)>, anyhow::Error> {
        self.query_call_count.fetch_add(1, Ordering::SeqCst);
        self.semantic_search_call_count.fetch_add(1, Ordering::SeqCst);
        let map = self.inner.lock().unwrap();
        let mut all: Vec<(f32, MemoryEntry)> = map
            .values()
            .cloned()
            .filter(|e| {
                let match_user = user_id
                    .map(|u| e.metadata.user_id.as_deref() == Some(u))
                    .unwrap_or(true);
                let match_conv = conversation_id
                    .map(|c| e.metadata.conversation_id.as_deref() == Some(c))
                    .unwrap_or(true);
                match_user && match_conv
            })
            .map(|e| (1.0_f32, e))
            .collect();
        all.truncate(limit);
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_add_and_search_by_user_and_conversation_updates_counters() {
        let store = MockMemoryStore::new();

        assert_eq!(store.get_store_call_count(), 0);
        assert_eq!(store.get_query_call_count(), 0);

        let metadata = MemoryMetadata {
            user_id: Some("user1".to_string()),
            conversation_id: Some("conv1".to_string()),
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };

        let entry = MemoryEntry::new("hello".to_string(), metadata);

        store.add(entry.clone()).await.unwrap();
        assert_eq!(store.get_store_call_count(), 1);

        let by_user = store.search_by_user("user1").await.unwrap();
        assert_eq!(store.get_query_call_count(), 1);
        assert_eq!(by_user.len(), 1);

        let by_conv = store.search_by_conversation("conv1").await.unwrap();
        assert_eq!(store.get_query_call_count(), 2);
        assert_eq!(by_conv.len(), 1);

        let sem = store.semantic_search(&[], 10, None, None).await.unwrap();
        assert_eq!(store.get_query_call_count(), 3);
        assert_eq!(store.get_semantic_search_call_count(), 1);
        assert_eq!(sem.len(), 1);
    }
}
