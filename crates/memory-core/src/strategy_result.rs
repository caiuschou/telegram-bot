//! # Strategy Result
//!
//! Result type returned by context strategies when building conversation context.
//! Consumed by ContextBuilder in the memory crate.

/// Category of messages returned by a strategy.
///
/// Used to distinguish "recent conversation" (main dialogue) from
/// "semantic search" (retrieved reference) in the AI prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageCategory {
    /// Recent conversation messages — main dialogue record for the AI.
    Recent,
    /// Semantically retrieved messages — reference context for the current query.
    Semantic,
}

/// Result type for context strategies.
///
/// Each strategy returns one of:
/// - **Messages**: List of formatted conversation messages with a category (Recent or Semantic)
/// - **Preferences**: Extracted user preferences string
/// - **Empty**: No content to add
#[derive(Debug, Clone)]
pub enum StrategyResult {
    /// Formatted conversation messages with category (e.g. "User: hello", "Assistant: hi").
    /// Category indicates whether they are recent dialogue or semantic search results.
    Messages {
        category: MessageCategory,
        messages: Vec<String>,
    },
    /// Extracted user preferences (e.g. "User Preferences: I like tea")
    Preferences(String),
    /// No content from this strategy
    Empty,
}
