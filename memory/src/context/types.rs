//! Context and metadata types for AI conversation.
//!
//! Defines the constructed context structure and its metadata, plus formatting
//! methods for AI model consumption. External: AI model APIs, prompt crate.

use chrono::{DateTime, Utc};

/// Represents a constructed context for AI conversation.
///
/// Contains all the information needed to provide context to an AI model,
/// including system instructions, conversation history, and user preferences.
///
/// # External Interactions
///
/// - **AI Models**: Formatted context is sent to LLM APIs (OpenAI, Anthropic, etc.)
/// - **Memory Management**: Context size must fit within model's token limit
/// - **Conversation State**: Maintains continuity across multi-turn conversations
///
/// # Components
///
/// - system_message: Optional AI personality/behavior instructions
/// - recent_messages: Main dialogue record (recent conversation) for the AI
/// - semantic_messages: Retrieved reference context from semantic search
/// - user_preferences: Extracted user preferences for personalization
/// - metadata: Context metadata including token counts and timestamps
#[derive(Debug, Clone)]
pub struct Context {
    /// System message if provided
    pub system_message: Option<String>,
    /// Recent conversation messages — main dialogue record for the AI.
    pub recent_messages: Vec<String>,
    /// Semantically retrieved messages — reference context for the current query.
    pub semantic_messages: Vec<String>,
    /// User preferences extracted from history
    pub user_preferences: Option<String>,
    /// Metadata about the context
    pub metadata: ContextMetadata,
}

/// Metadata about the constructed context.
///
/// Provides diagnostic information about the context, useful for monitoring,
/// debugging, and ensuring context stays within token limits.
///
/// # External Interactions
///
/// - **Monitoring**: Metadata can be logged for observability
/// - **Token Management**: total_tokens helps prevent exceeding API limits
/// - **Analytics**: message_count and timestamps enable usage analysis
#[derive(Debug, Clone)]
pub struct ContextMetadata {
    /// User ID for this context
    pub user_id: Option<String>,
    /// Conversation ID for this context
    pub conversation_id: Option<String>,
    /// Total estimated token count
    pub total_tokens: usize,
    /// Number of messages in context
    pub message_count: usize,
    /// When the context was built
    pub created_at: DateTime<Utc>,
}

impl Context {
    /// Returns context as a single string for AI models (no current question).
    ///
    /// Delegates to `prompt::format_for_model`. Used when only the context block is needed
    /// (e.g. handler returning context string). External: output sent to LLM APIs.
    pub fn format_for_model(&self, include_system: bool) -> String {
        prompt::format_for_model(
            include_system,
            self.system_message.as_deref(),
            self.user_preferences.as_deref(),
            &self.recent_messages,
            &self.semantic_messages,
        )
    }

    /// Returns context as chat messages with different types (system, user, assistant).
    ///
    /// Calls `prompt::format_for_model_as_messages_with_roles` so recent conversation
    /// lines ("User: ...", "Assistant: ...", "System: ...") become separate `ChatMessage`
    /// with matching roles. Order: optional System, parsed recent (User/Assistant/System),
    /// optional User(preferences+semantic block), User(question).
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

    /// Returns true if there are no recent and no semantic messages.
    pub fn is_empty(&self) -> bool {
        self.recent_messages.is_empty() && self.semantic_messages.is_empty()
    }

    /// Checks if the context exceeds the token limit.
    pub fn exceeds_limit(&self, limit: usize) -> bool {
        self.metadata.total_tokens > limit
    }
}
