//! Memory storage trait.

use async_trait::async_trait;
use uuid::Uuid;

use super::types::MemoryEntry;

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>;
    async fn get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error>;
    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>;
    async fn delete(&self, id: Uuid) -> Result<(), anyhow::Error>;
    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error>;
    async fn search_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MemoryEntry>, anyhow::Error>;
    async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        user_id: Option<&str>,
        conversation_id: Option<&str>,
    ) -> Result<Vec<(f32, MemoryEntry)>, anyhow::Error>;
}
