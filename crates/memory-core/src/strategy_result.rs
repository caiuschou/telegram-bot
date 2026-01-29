//! # Strategy Result
//!
//! Result type returned by context strategies when building conversation context.
//! Consumed by ContextBuilder in the memory crate.

/// Result type for context strategies.
///
/// Each strategy returns one of:
/// - **Messages**: List of formatted conversation messages
/// - **Preferences**: Extracted user preferences string
/// - **Empty**: No content to add
#[derive(Debug, Clone)]
pub enum StrategyResult {
    /// Formatted conversation messages (e.g. "User: hello", "Assistant: hi")
    Messages(Vec<String>),
    /// Extracted user preferences (e.g. "User Preferences: I like tea")
    Preferences(String),
    /// No content from this strategy
    Empty,
}
