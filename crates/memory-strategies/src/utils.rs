//! Shared utilities for context strategies.
//!
//! Provides message formatting and preference extraction used by multiple strategies.
//! External interactions: AI models consume formatted messages; memory store provides raw entries.

use memory_core::{MemoryEntry, MemoryRole};

/// Maximum character length for content in logs (avoids dumping huge strings).
pub(crate) const MAX_LOG_CONTENT_LEN: usize = 400;

/// Truncates a string for logging; appends "..." if truncated.
pub(crate) fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Formats a memory entry as a message string.
///
/// Converts a MemoryEntry into a human-readable message format suitable for
/// inclusion in AI conversation context. The format follows standard conversation
/// conventions with role prefixes.
///
/// # Format Pattern
///
/// "{Role}: {content}"
///
/// Where Role is one of: "User", "Assistant", "System"
///
/// # External Interactions
///
/// - **AI Models**: Formatted messages are directly consumed by LLM APIs
/// - **Conversation Parsers**: Follows standard role-based message format
pub(crate) fn format_message(entry: &MemoryEntry) -> String {
    let role = match entry.metadata.role {
        MemoryRole::User => "User",
        MemoryRole::Assistant => "Assistant",
        MemoryRole::System => "System",
    };

    format!("{}: {}", role, entry.content)
}

/// Extracts user preferences from conversation history.
///
/// Analyzes historical messages to identify and extract user preferences expressed
/// during conversations. Uses simple pattern matching to detect preference statements.
///
/// # Detection Patterns
///
/// Scans for phrases indicating preferences:
/// - "I like" followed by content
/// - "I prefer" followed by content
///
/// # External Interactions
///
/// - **Memory Store**: Reads historical conversation data
/// - **AI Context**: Extracted preferences are included in AI conversation context
/// - **User Personalization**: Enables personalized AI responses based on preferences
pub(crate) fn extract_preferences(entries: &[MemoryEntry]) -> Vec<String> {
    let mut preferences = Vec::new();

    for entry in entries {
        let content = entry.content.to_lowercase();

        if content.contains("i like") || content.contains("i prefer") {
            if let Some(start) = content.find("i like") {
                let preference = &content[start..];
                preferences.push(preference.to_string());
            } else if let Some(start) = content.find("i prefer") {
                let preference = &content[start..];
                preferences.push(preference.to_string());
            }
        }
    }

    preferences
}
