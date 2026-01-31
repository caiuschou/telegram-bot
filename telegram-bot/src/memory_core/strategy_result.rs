//! Result type for context strategies.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageCategory {
    Recent,
    Semantic,
}

#[derive(Debug, Clone)]
pub enum StrategyResult {
    Messages {
        category: MessageCategory,
        messages: Vec<String>,
    },
    Preferences(String),
    Empty,
}
