//! User preferences context strategy.
//!
//! Extracts user preferences from conversation history for personalization.
//! External interactions: MemoryStore.search_by_user; AI personalization.

use async_trait::async_trait;
use memory_core::{MemoryStore, StrategyResult};
use tracing::debug;

use super::strategy::ContextStrategy;
use super::utils::extract_preferences;

/// Strategy for extracting user preferences from conversation history.
#[derive(Debug, Clone)]
pub struct UserPreferencesStrategy;

impl UserPreferencesStrategy {
    /// Creates a new UserPreferencesStrategy.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ContextStrategy for UserPreferencesStrategy {
    /// Builds context by extracting user preferences from conversation history.
    ///
    /// Analyzes all historical messages for a user to identify expressed
    /// preferences, which can be used to personalize AI responses.
    ///
    /// # Process
    ///
    /// 1. Retrieve all entries for the user from MemoryStore
    /// 2. Analyze messages for preference indicators ("I like", "I prefer")
    /// 3. Extract preference statements from matched messages
    /// 4. Return formatted preferences string or Empty if none found
    ///
    /// # External Interactions
    ///
    /// - **MemoryStore**: Queries all historical messages for the user
    /// - **Storage**: Reads persistent conversation history
    /// - **AI Personalization**: Extracted preferences enable personalized responses
    /// - **User Experience**: AI can recall and respect user preferences across sessions
    ///
    /// # Limitations
    ///
    /// - Uses simple pattern matching (not semantic analysis)
    /// - May miss preferences expressed in complex language
    /// - May include false positives in edge cases
    /// - Future: Use LLM to extract preferences more accurately
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
                debug!(
                    "UserPreferencesStrategy: no user_id provided, returning Empty"
                );
                return Ok(StrategyResult::Empty);
            }
        };

        let entries = store.search_by_user(user_id).await?;

        debug!(
            user_id = user_id,
            entry_count = entries.len(),
            "UserPreferencesStrategy: loaded entries for preference extraction"
        );

        let preferences = extract_preferences(&entries);

        if preferences.is_empty() {
            debug!(
                user_id = user_id,
                "UserPreferencesStrategy: no preferences detected, returning Empty"
            );
            Ok(StrategyResult::Empty)
        } else {
            debug!(
                user_id = user_id,
                preference_count = preferences.len(),
                "UserPreferencesStrategy: extracted user preferences"
            );
            Ok(StrategyResult::Preferences(format!(
                "User Preferences: {}",
                preferences.join(", ")
            )))
        }
    }
}
