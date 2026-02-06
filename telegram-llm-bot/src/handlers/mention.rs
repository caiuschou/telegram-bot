//! Shared pure functions for @-mention detection and question extraction.
//! Used by [`super::sync_llm::SyncLLMHandler`] and [`super::mention_detector::LLMDetectionHandler`].

/// Returns true if `text` contains a @mention of the given bot username.
#[inline]
pub(crate) fn is_bot_mentioned(text: &str, bot_username: &str) -> bool {
    text.contains(&format!("@{}", bot_username))
}

/// Strips the bot @mention from `text` and returns the trimmed string (the question for the LLM).
#[inline]
pub(crate) fn extract_question(text: &str, bot_username: &str) -> String {
    text.replace(&format!("@{}", bot_username), "")
        .trim()
        .to_string()
}
