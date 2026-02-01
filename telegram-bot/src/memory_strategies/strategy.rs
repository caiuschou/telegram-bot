//! Context building strategy trait.

use async_trait::async_trait;
use crate::memory_core::{MemoryStore, StrategyResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKind {
    Primary,
    Recent,
}

#[async_trait]
pub trait ContextStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn store_kind(&self) -> StoreKind {
        StoreKind::Primary
    }
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        conversation_id: &Option<String>,
        query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error>;
}
