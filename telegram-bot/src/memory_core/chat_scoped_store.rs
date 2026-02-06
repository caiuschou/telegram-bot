//! Chat-scoped view of a [`MemoryStore`]: all operations are restricted to a single `chat_id`
//! (used as `conversation_id`). See plan-memory-store-get-store and plan-langgraph-bot-inject-vectorstore.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use super::store::MemoryStore;
use super::types::MemoryEntry;

/// Wrapper around a [`MemoryStore`] that fixes all operations to a single chat (conversation).
/// Used so that tools and agents cannot accidentally query or write to other chats.
#[derive(Clone)]
pub struct ChatScopedStore {
    inner: Arc<dyn MemoryStore>,
    chat_id: String,
}

impl ChatScopedStore {
    /// Creates a new chat-scoped view. All `semantic_search` and `search_by_conversation` calls
    /// use `chat_id`; `add`/`update` set `entry.metadata.conversation_id` to `chat_id`.
    pub fn new(inner: Arc<dyn MemoryStore>, chat_id: impl Into<String>) -> Self {
        Self {
            inner,
            chat_id: chat_id.into(),
        }
    }
}

#[async_trait]
impl MemoryStore for ChatScopedStore {
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entry = entry;
        entry.metadata.conversation_id = Some(self.chat_id.clone());
        self.inner.add(entry).await
    }

    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        self.inner.get(id).await
    }

    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entry = entry;
        entry.metadata.conversation_id = Some(self.chat_id.clone());
        self.inner.update(entry).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error> {
        self.inner.delete(id).await
    }

    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        self.inner.search_by_user(user_id).await
    }

    async fn search_by_conversation(
        &self,
        _conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        self.inner.search_by_conversation(&self.chat_id).await
    }

    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        user_id: Option<&str>,
        _conversation_id: Option<&str>,
    ) -> Result<Vec<(f32, MemoryEntry)>, anyhow::Error> {
        self.inner
            .semantic_search(query_embedding, limit, user_id, Some(&self.chat_id))
            .await
    }
}

/// Returns a [`MemoryStore`] that restricts all operations to the given `chat_id`
/// (as `conversation_id`). Use this when injecting a store for a single chat so that
/// tools and agents cannot access other chats.
pub fn get_store(
    inner: Arc<dyn MemoryStore>,
    chat_id: impl Into<String>,
) -> Arc<dyn MemoryStore> {
    Arc::new(ChatScopedStore::new(inner, chat_id))
}
