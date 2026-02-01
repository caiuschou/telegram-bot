//! # Prompt
//!
//! Formats structured context into a single prompt string for AI models.
//!
//! ## Format
//!
//! - **System** (optional): `System: {message}`
//! - **User Preferences** (optional): `User Preferences: {preferences}`
//! - **Conversation (recent)**: Section title + main dialogue messages
//! - **Relevant reference (semantic)**: Section title + retrieved reference messages
//!
//! ## Usage
//!
//! Used by the `memory` crate when calling `Context::format_for_model()`.
//! Can also be used directly with any source of system message, preferences,
//! and message lists (e.g. from other context builders).
//!
//! ## External interactions
//!
//! - **AI models**: Output is sent to LLM APIs (OpenAI, Anthropic, etc.).

/// Role of a message, one-to-one with OpenAI Chat Completions API `role` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// System instruction (API `role: "system"`).
    System,
    /// User message (API `role: "user"`).
    User,
    /// Assistant message (API `role: "assistant"`).
    Assistant,
}

/// A single chat message, one-to-one with one element of OpenAI `messages` array.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// Default system instruction when no custom system message is provided.
/// Used when callers want a fixed system prompt; pass this as `system_message` or use as fallback.
pub const DEFAULT_SYSTEM_MESSAGE: &str = "You are a helpful assistant.";

/// Section title for the main dialogue (recent conversation).
pub const SECTION_RECENT: &str = "Conversation (recent):";

/// Section title for semantically retrieved reference messages.
pub const SECTION_SEMANTIC: &str = "Relevant reference (semantic):";

/// Builds context as a single string for AI models (no current question).
///
/// Used when only the context block is needed (e.g. handler returning context string).
/// Order: optional System (if include_system), User Preferences, Conversation (recent), Relevant reference (semantic).
///
/// # Arguments
///
/// * `include_system` - If true and system_message is present, prepend "System: {message}\n\n"
/// * `system_message` - Optional system instruction
/// * `user_preferences` - Optional user preferences (section "User Preferences: ...")
/// * `recent_messages` - Historical conversation lines (section "Conversation (recent):")
/// * `semantic_messages` - Semantic reference lines (section "Relevant reference (semantic):")
///
/// # Returns
///
/// A single string suitable for inclusion in prompts. External: consumed by LLM APIs.
pub fn format_for_model<R, S, RI, SI>(
    include_system: bool,
    system_message: Option<&str>,
    user_preferences: Option<&str>,
    recent_messages: R,
    semantic_messages: S,
) -> String
where
    R: IntoIterator<Item = RI>,
    RI: AsRef<str>,
    S: IntoIterator<Item = SI>,
    SI: AsRef<str>,
{
    let mut out = String::new();
    if include_system {
        if let Some(msg) = system_message {
            out.push_str("System: ");
            out.push_str(msg);
            out.push_str("\n\n");
        }
    }
    if let Some(prefs) = user_preferences {
        out.push_str("User Preferences: ");
        out.push_str(prefs);
        out.push_str("\n\n");
    }
    let recent: Vec<String> = recent_messages
        .into_iter()
        .map(|r| r.as_ref().to_string())
        .collect();
    if !recent.is_empty() {
        out.push_str(SECTION_RECENT);
        out.push('\n');
        for line in &recent {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }
    let semantic: Vec<String> = semantic_messages
        .into_iter()
        .map(|s| s.as_ref().to_string())
        .collect();
    if !semantic.is_empty() {
        out.push_str(SECTION_SEMANTIC);
        out.push('\n');
        for line in &semantic {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Alias for `format_for_model_as_messages_with_roles`: same parameters and behavior.
/// Returns messages with correct roles for OpenAI API, including the current question as last User message.
pub fn format_for_model_as_messages<R, S, RI, SI>(
    include_system: bool,
    system_message: Option<&str>,
    user_preferences: Option<&str>,
    recent_messages: R,
    semantic_messages: S,
    current_question: &str,
) -> Vec<ChatMessage>
where
    R: IntoIterator<Item = RI>,
    RI: AsRef<str>,
    S: IntoIterator<Item = SI>,
    SI: AsRef<str>,
{
    format_for_model_as_messages_with_roles(
        include_system,
        system_message,
        user_preferences,
        recent_messages,
        semantic_messages,
        current_question,
    )
}

/// Parses a single line in "Role: content" form into a `ChatMessage`.
///
/// Supports "User: ...", "Assistant: ...", "System: ...". Content is trimmed.
/// Returns `None` for empty lines or unknown prefixes.
pub fn parse_message_line(line: &str) -> Option<ChatMessage> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if let Some(content) = line.strip_prefix("User:") {
        return Some(ChatMessage::user(content.trim()));
    }
    if let Some(content) = line.strip_prefix("Assistant:") {
        return Some(ChatMessage::assistant(content.trim()));
    }
    if let Some(content) = line.strip_prefix("System:") {
        return Some(ChatMessage::system(content.trim()));
    }
    None
}

/// Builds context as a list of messages with correct roles (system, user, assistant).
///
/// Recent messages are treated as historical conversation content and formatted as one
/// User message (section "Conversation (recent):"). Optional system, optional User(recent block),
/// optional User(preferences+semantic block), and User(current_question) complete the list.
///
/// # Order
///
/// Optional system → optional User(recent conversation block) → optional User(preferences+semantic block) → User(current_question).
///
/// # Arguments
///
/// * `_include_system` - Reserved for future use (ignored; system is pushed when system_message is present)
/// * `system_message` - Optional system instruction; if present, pushed as first message (System)
/// * `user_preferences` - Optional user preferences (included in context block with semantic)
/// * `recent_messages` - Historical conversation lines (formatted as one User message with section title)
/// * `semantic_messages` - Semantic reference lines (included in context block after recent)
/// * `current_question` - Current user question (last message is User)
///
/// # Returns
///
/// `Vec<ChatMessage>` with mixed roles (system, user, assistant) for OpenAI API.
pub fn format_for_model_as_messages_with_roles<R, S, RI, SI>(
    _include_system: bool,
    system_message: Option<&str>,
    user_preferences: Option<&str>,
    recent_messages: R,
    semantic_messages: S,
    current_question: &str,
) -> Vec<ChatMessage>
where
    R: IntoIterator<Item = RI>,
    RI: AsRef<str>,
    S: IntoIterator<Item = SI>,
    SI: AsRef<str>,
{
    let mut messages = Vec::new();

    if let Some(msg) = system_message {
        messages.push(ChatMessage::system(msg));
    }

    let recent: Vec<String> = recent_messages
        .into_iter()
        .map(|r| r.as_ref().to_string())
        .collect();
    let semantic: Vec<String> = semantic_messages
        .into_iter()
        .map(|s| s.as_ref().to_string())
        .collect();

    // Single context User message: preferences + recent block + semantic block (test expects 2 msgs: context + question)
    let mut context_block = String::new();
    if let Some(prefs) = user_preferences {
        context_block.push_str("User Preferences: ");
        context_block.push_str(prefs);
        context_block.push_str("\n\n");
    }
    if !recent.is_empty() {
        context_block.push_str(SECTION_RECENT);
        context_block.push('\n');
        for line in &recent {
            context_block.push_str(line);
            context_block.push('\n');
        }
        context_block.push('\n');
    }
    if !semantic.is_empty() {
        context_block.push_str(SECTION_SEMANTIC);
        context_block.push('\n');
        for line in &semantic {
            context_block.push_str(line);
            context_block.push('\n');
        }
    }
    if !context_block.is_empty() {
        messages.push(ChatMessage::user(context_block));
    }

    messages.push(ChatMessage::user(current_question));
    messages
}
