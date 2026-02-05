//! Memory summary for checkpoint (short-term memory) and in-memory message preview.
//!
//! Used by `memory` subcommand and by load/seed to preview messages without writing to checkpoint.

use anyhow::Result;
use langgraph_bot::{
    format_thread_summary, get_react_state_from_checkpointer, list_thread_ids,
};

pub const MEMORY_PREVIEW_LEN: usize = 50;

/// Prints message count and first `limit` messages (no checkpoint). Used by load/seed after resolving or generating messages.
pub fn print_messages_preview(messages: &[langgraph::Message], limit: usize) {
    println!("Messages: {} (preview, not imported into checkpoint)", messages.len());
    for (i, msg) in messages.iter().take(limit).enumerate() {
        let preview = match msg {
            langgraph::Message::User(s) => format!("User: {}", s.chars().take(40).collect::<String>()),
            langgraph::Message::Assistant(s) => format!("Assistant: {}", s.chars().take(40).collect::<String>()),
            _ => "Other".into(),
        };
        println!("  [{}] {}", i + 1, preview);
    }
    if messages.len() > limit {
        println!("  ... and {} more", messages.len() - limit);
    }
}

/// Prints short-term memory (checkpoint) summary: either one thread or all threads with message count and previews.
pub async fn print_memory_summary(db: &std::path::Path, thread_id: Option<&str>) -> Result<()> {
    println!("Short-term memory (checkpoint): {}", db.display());
    if let Some(tid) = thread_id {
        let state = get_react_state_from_checkpointer(db, tid).await?;
        println!(
            "{}\n",
            format_thread_summary(tid, &state, MEMORY_PREVIEW_LEN)
        );
        return Ok(());
    }
    let ids = list_thread_ids(db)?;
    if ids.is_empty() {
        println!("  (no threads)");
        return Ok(());
    }
    println!("  threads: {}", ids.len());
    for tid in &ids {
        let state = get_react_state_from_checkpointer(db, tid).await?;
        println!(
            "{}\n",
            format_thread_summary(tid, &state, MEMORY_PREVIEW_LEN)
        );
    }
    Ok(())
}

