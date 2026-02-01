//! Context and metadata types for AI conversation.

use chrono::{DateTime, Utc};

/// Represents a constructed context for AI conversation.
#[derive(Debug, Clone)]
pub struct Context {
    pub system_message: Option<String>,
    pub recent_messages: Vec<String>,
    pub semantic_messages: Vec<String>,
    pub user_preferences: Option<String>,
    pub metadata: ContextMetadata,
}

/// Metadata about the constructed context.
#[derive(Debug, Clone)]
pub struct ContextMetadata {
    pub user_id: Option<String>,
    pub conversation_id: Option<String>,
    pub total_tokens: usize,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
}

impl Context {
    pub fn format_for_model(&self, include_system: bool) -> String {
        prompt::format_for_model(
            include_system,
            self.system_message.as_deref(),
            self.user_preferences.as_deref(),
            &self.recent_messages,
            &self.semantic_messages,
        )
    }

    pub fn to_messages(
        &self,
        include_system: bool,
        current_question: &str,
    ) -> Vec<prompt::ChatMessage> {
        prompt::format_for_model_as_messages_with_roles(
            include_system,
            self.system_message.as_deref(),
            self.user_preferences.as_deref(),
            &self.recent_messages,
            &self.semantic_messages,
            current_question,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.recent_messages.is_empty() && self.semantic_messages.is_empty()
    }

    pub fn exceeds_limit(&self, limit: usize) -> bool {
        self.metadata.total_tokens > limit
    }
}
