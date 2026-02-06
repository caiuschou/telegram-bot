//! Shared pure functions for @-mention detection and question extraction.
//!
//! Used by `telegram-llm-bot` (e.g. `InlineLLMHandler` and `LLMDetectionHandler`)
//! and `langgraph-bot` (e.g. `AgentHandler`) to decide when a message triggers a bot reply and to
//! extract the user question from @mention text.

use crate::Message;

/// Returns true if `text` contains a @mention of the given bot username.
#[inline]
pub fn is_bot_mentioned(text: &str, bot_username: &str) -> bool {
    text.contains(&format!("@{}", bot_username))
}

/// Strips the bot @mention from `text` and returns the trimmed string (the question for downstream handlers).
#[inline]
pub fn extract_question(text: &str, bot_username: &str) -> String {
    text.replace(&format!("@{}", bot_username), "")
        .trim()
        .to_string()
}

/// Resolves the user question if the message triggers a reply (reply-to-bot or @mention).
///
/// - **Reply-to-bot**: returns `Some(message.content)`.
/// - **@mention with non-empty text**: returns `Some(extract_question(content, username))`.
/// - **@mention with empty text**: if `empty_mention_default` is `Some(s)` returns `Some(s.to_string())`, else `None`.
/// - Otherwise returns `None`.
/// Default prompt when user only @mentions with no text; use as `empty_mention_default` for [`get_question`].
pub const DEFAULT_EMPTY_MENTION_PROMPT: &str =
    "The user only @mentioned you with no specific question. Please greet them briefly and invite them to ask.";

pub fn get_question(
    message: &Message,
    bot_username: Option<&str>,
    empty_mention_default: Option<&str>,
) -> Option<String> {
    if message.reply_to_message_id.is_some() && message.reply_to_message_from_bot {
        return Some(message.content.clone());
    }
    if let Some(username) = bot_username {
        if is_bot_mentioned(&message.content, username) {
            let q = extract_question(&message.content, username);
            if !q.is_empty() {
                return Some(q);
            }
            if let Some(default) = empty_mention_default {
                return Some(default.to_string());
            }
            return None;
        }
    }
    None
}
