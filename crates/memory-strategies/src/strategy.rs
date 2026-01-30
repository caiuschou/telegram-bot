//! Context building strategy trait.
//!
//! Defines the interface that all context strategies implement.
//! External interactions: memory-core MemoryStore/StrategyResult; callers build_context.

use async_trait::async_trait;
use memory_core::{MemoryStore, StrategyResult};

/// Which store the strategy uses when ContextBuilder has both primary and recent store.
///
/// When `recent_store` is set (e.g. SQLite for recent messages), strategies with
/// `StoreKind::Recent` use it; others use the primary store (e.g. Lance for semantic search).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKind {
    /// Use the primary store (e.g. Lance). Used by SemanticSearchStrategy.
    Primary,
    /// Use the recent store when provided (e.g. SQLite). Used by RecentMessagesStrategy and UserPreferencesStrategy.
    Recent,
}

/// Trait for context building strategies.
#[async_trait]
pub trait ContextStrategy: Send + Sync {
    /// Returns the strategy name for logging and diagnostics.
    fn name(&self) -> &str;

    /// Which store to use when both primary and recent store are available.
    /// Default: Primary (single-store behavior).
    fn store_kind(&self) -> StoreKind {
        StoreKind::Primary
    }

    /// Builds context using strategy.
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        conversation_id: &Option<String>,
        query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error>;
}
