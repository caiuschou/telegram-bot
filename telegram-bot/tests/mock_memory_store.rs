//! MockMemoryStore 实现
//!
//! - 提供一个简单的基于内存的 `MemoryStore` 实现，仅用于测试环境。
//! - 使用 `HashMap<Uuid, MemoryEntry>` 存储数据，支持按用户和会话查询。
//! - `semantic_search` 当前实现为截断后的全量返回，后续可按需扩展为真正的相似度搜索。

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use async_trait::async_trait;
use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use uuid::Uuid;

/// 简单的内存实现，用于在测试中替代真实向量存储。
///
/// 特性：
/// - 使用 `HashMap<Uuid, MemoryEntry>` 存储数据。
/// - 提供按用户、会话 ID 的查询能力。
/// - `semantic_search` 目前实现为简单返回全部数据（按插入顺序）。
/// - 通过计数器跟踪 `add`、各类查询及 `semantic_search` 调用次数，便于在测试中进行断言。
#[derive(Debug, Clone)]
pub struct MockMemoryStore {
    inner: Arc<Mutex<HashMap<Uuid, MemoryEntry>>>,
    store_call_count: Arc<AtomicUsize>,
    query_call_count: Arc<AtomicUsize>,
    /// 仅当调用 `semantic_search` 时递增；用于断言「embed + 向量检索」完整执行。
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
    /// 创建新的空 `MockMemoryStore` 实例。
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取存储调用次数（例如用户消息和 AI 回复被写入的次数）。
    pub fn get_store_call_count(&self) -> usize {
        self.store_call_count.load(Ordering::SeqCst)
    }

    /// 获取查询调用次数（search_by_user / search_by_conversation / semantic_search 任一调用都会递增）。
    pub fn get_query_call_count(&self) -> usize {
        self.query_call_count.load(Ordering::SeqCst)
    }

    /// 获取语义检索调用次数（仅 `semantic_search` 调用时递增；embed 完成后才会调用）。
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
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        self.query_call_count.fetch_add(1, Ordering::SeqCst);
        self.semantic_search_call_count.fetch_add(1, Ordering::SeqCst);
        let map = self.inner.lock().unwrap();
        let mut all: Vec<MemoryEntry> = map.values().cloned().collect();
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

        let sem = store.semantic_search(&[], 10).await.unwrap();
        assert_eq!(store.get_query_call_count(), 3);
        assert_eq!(store.get_semantic_search_call_count(), 1);
        assert_eq!(sem.len(), 1);
    }
}
