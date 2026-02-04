//! Load messages using seed-messages and convert to langgraph `Message` list.
//!
//! Uses `seed_messages::SeedMessage` for parsing (same shape as messages.json).
//! Convention: `direction == "received"` → User(content), `"sent"` → Assistant(content).
//! With-user-info variants prefix User message content with `[User: first_name / @username]` via `format::user_info_prefix`.

use anyhow::Result;
use langgraph::Message;
use seed_messages::SeedMessage;

use crate::format::user_info_prefix;

/// Converts `Vec<SeedMessage>` to `Vec<Message>`.
/// - `direction == "received"` → `Message::User(content)`
/// - `direction == "sent"` → `Message::Assistant(content)`
/// Other directions are skipped. Result is typically passed to `checkpoint::import_messages_into_checkpointer`.
pub fn seed_messages_to_messages(seed: Vec<SeedMessage>) -> Vec<Message> {
    seed_messages_to_messages_with_stats(seed).0
}

/// Converts `Vec<SeedMessage>` to `Vec<Message>` and returns the number of skipped messages.
/// - `direction == "received"` → `Message::User(content)`
/// - `direction == "sent"` → `Message::Assistant(content)`
/// Other directions are skipped; their count is returned as the second element.
pub fn seed_messages_to_messages_with_stats(seed: Vec<SeedMessage>) -> (Vec<Message>, usize) {
    let mut skipped = 0;
    let messages: Vec<Message> = seed
        .into_iter()
        .filter_map(|r| match r.direction.as_str() {
            "received" => Some(Message::User(r.content)),
            "sent" => Some(Message::Assistant(r.content)),
            _ => {
                skipped += 1;
                None
            }
        })
        .collect();
    (messages, skipped)
}

/// Converts `Vec<SeedMessage>` to `Vec<Message>` with user identity prefixed on each User message.
///
/// - `direction == "received"` → `Message::User(prefix + content)` where prefix is from `format::user_info_prefix(first_name, last_name, username)`.
/// - `direction == "sent"` → `Message::Assistant(content)` (no prefix).
/// Other directions are skipped. Result is typically passed to `checkpoint::import_messages_into_checkpointer`.
pub fn seed_messages_to_messages_with_user_info(seed: Vec<SeedMessage>) -> Vec<Message> {
    seed_messages_to_messages_with_user_info_with_stats(seed).0
}

/// Same as `seed_messages_to_messages_with_user_info` but returns the number of skipped messages (non-received/sent directions).
pub fn seed_messages_to_messages_with_user_info_with_stats(
    seed: Vec<SeedMessage>,
) -> (Vec<Message>, usize) {
    let mut skipped = 0;
    let messages: Vec<Message> = seed
        .into_iter()
        .filter_map(|r| match r.direction.as_str() {
            "received" => {
                let prefix = user_info_prefix(
                    r.first_name.as_deref(),
                    r.last_name.as_deref(),
                    r.username.as_deref(),
                );
                Some(Message::User(format!("{}{}", prefix, r.content)))
            }
            "sent" => Some(Message::Assistant(r.content)),
            _ => {
                skipped += 1;
                None
            }
        })
        .collect();
    (messages, skipped)
}

/// Reads JSON array from path (e.g. messages.json) as `Vec<SeedMessage>` and converts to `Vec<Message>`.
/// Delegates to `load_messages_from_path_with_stats`; use that if you need the skipped count.
pub fn load_messages_from_path(path: impl AsRef<std::path::Path>) -> Result<Vec<Message>> {
    load_messages_from_path_with_stats(path).map(|(m, _)| m)
}

/// Reads JSON array from path and converts to `Vec<Message>`, returning the number of skipped messages.
pub fn load_messages_from_path_with_stats(
    path: impl AsRef<std::path::Path>,
) -> Result<(Vec<Message>, usize)> {
    let bytes = std::fs::read(path)?;
    load_messages_from_slice_with_stats(&bytes)
}

/// Parses JSON array from bytes as `Vec<SeedMessage>` and converts to `Vec<Message>`.
/// Delegates to `load_messages_from_slice_with_stats`; use that if you need the skipped count.
pub fn load_messages_from_slice(bytes: &[u8]) -> Result<Vec<Message>> {
    load_messages_from_slice_with_stats(bytes).map(|(m, _)| m)
}

/// Parses JSON array from bytes and converts to `Vec<Message>`, returning the number of skipped messages.
pub fn load_messages_from_slice_with_stats(bytes: &[u8]) -> Result<(Vec<Message>, usize)> {
    let raw: Vec<SeedMessage> = serde_json::from_slice(bytes)?;
    Ok(seed_messages_to_messages_with_stats(raw))
}

/// Parses JSON array from bytes and converts to `Vec<Message>` with user info prefix on User messages.
pub fn load_messages_from_slice_with_user_info(bytes: &[u8]) -> Result<Vec<Message>> {
    load_messages_from_slice_with_user_info_with_stats(bytes).map(|(m, _)| m)
}

/// Parses JSON array from bytes and converts to `Vec<Message>` with user info prefix, returning the number of skipped messages.
pub fn load_messages_from_slice_with_user_info_with_stats(
    bytes: &[u8],
) -> Result<(Vec<Message>, usize)> {
    let raw: Vec<SeedMessage> = serde_json::from_slice(bytes)?;
    Ok(seed_messages_to_messages_with_user_info_with_stats(raw))
}
