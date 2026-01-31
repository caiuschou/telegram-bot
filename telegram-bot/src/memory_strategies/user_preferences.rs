//! User preferences context strategy.

use async_trait::async_trait;
use crate::memory_core::{MemoryStore, StrategyResult};
use tracing::{debug, info};

use super::strategy::{ContextStrategy, StoreKind};
use super::utils::extract_preferences;

#[derive(Debug, Clone)]
pub struct UserPreferencesStrategy;

impl UserPreferencesStrategy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextStrategy for UserPreferencesStrategy {
    fn name(&self) -> &str {
        "UserPreferences"
    }
    fn store_kind(&self) -> StoreKind {
        StoreKind::Recent
    }
    async fn build_context(
        &self,
        store: &dyn MemoryStore,
        user_id: &Option<String>,
        _conversation_id: &Option<String>,
        _query: &Option<String>,
    ) -> Result<StrategyResult, anyhow::Error> {
        let user_id = match user_id {
            Some(uid) => uid,
            None => {
                debug!("UserPreferencesStrategy: no user_id provided, returning Empty");
                return Ok(StrategyResult::Empty);
            }
        };
        let entries = store.search_by_user(user_id).await.map_err(|e| {
            tracing::error!(error = %e, user_id = %user_id, "UserPreferencesStrategy: search_by_user failed");
            e
        })?;
        info!(user_id = %user_id, entry_count = entries.len(), "UserPreferencesStrategy: loaded entries for preference extraction");
        let preferences = extract_preferences(&entries);
        if preferences.is_empty() {
            debug!(user_id = %user_id, "UserPreferencesStrategy: no preferences detected, returning Empty");
            Ok(StrategyResult::Empty)
        } else {
            let prefs_str = format!("User Preferences: {}", preferences.join(", "));
            info!(user_id = %user_id, preference_count = preferences.len(), preferences = %prefs_str, "UserPreferencesStrategy: user preferences extracted");
            Ok(StrategyResult::Preferences(prefs_str))
        }
    }
}
