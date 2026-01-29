//! Context building strategy trait.
//!
//! Defines the interface that all context strategies implement.
//! External interactions: memory-core MemoryStore/StrategyResult; callers build_context.

use async_trait::async_trait;
use memory_core::{MemoryStore, StrategyResult};

/// Trait for context building strategies.
#[async_trait]
pub trait ContextStrategy: Send + Sync {
    /// Builds context using strategy.
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        conversation_id: &Option<String>,
        query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error>;
}
