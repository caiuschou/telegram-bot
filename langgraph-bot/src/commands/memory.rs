//! Memory summary and in-memory message preview.
//!
//! Short-term memory (checkpoint) is disabled; the `memory` subcommand reports that. Load/seed use this module for message preview only.

use anyhow::Result;
use langgraph::Message;

/// Default preview length for message preview (used by load/seed).
pub const MEMORY_PREVIEW_LEN: usize = 50;

/// Prints message count and first `limit` messages (no checkpoint). Used by load/seed after resolving or generating messages.
pub fn print_messages_preview(messages: &[Message], limit: usize) {
    println!("Messages: {} (preview, not imported into checkpoint)", messages.len());
    for (i, msg) in messages.iter().take(limit).enumerate() {
        let preview = match msg {
            Message::User(s) => format!("User: {}", s.chars().take(40).collect::<String>()),
            Message::Assistant(s) => format!("Assistant: {}", s.chars().take(40).collect::<String>()),
            _ => "Other".into(),
        };
        println!("  [{}] {}", i + 1, preview);
    }
    if messages.len() > limit {
        println!("  ... and {} more", messages.len() - limit);
    }
}

/// Prints that short-term memory is disabled (no checkpoint).
pub fn print_memory_summary() -> Result<()> {
    println!("Short-term memory is disabled.");
    Ok(())
}

