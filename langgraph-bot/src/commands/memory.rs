//! Memory summary and import preview for checkpoint (short-term memory).
//!
//! Used by `memory` subcommand and by load/seed after import.

use anyhow::Result;
use langgraph_bot::{
    format_thread_summary, get_messages_from_checkpointer, get_react_state_from_checkpointer,
    list_thread_ids, verify_messages_format, verify_messages_integrity,
};

pub const MEMORY_PREVIEW_LEN: usize = 50;

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

/// After import: verify checkpoint content and print a short preview (first 3 messages).
pub async fn print_import_preview(
    db: &std::path::Path,
    thread_id: &str,
    messages: &[langgraph::Message],
) -> Result<()> {
    let verified = get_messages_from_checkpointer(db, thread_id).await?;
    verify_messages_integrity(messages, &verified)?;
    verify_messages_format(&verified)?;
    println!("Integrity: OK ({} messages)", verified.len());
    println!("Format: OK (User/Assistant only)");
    for (i, msg) in verified.iter().take(3).enumerate() {
        let preview = match msg {
            langgraph::Message::User(s) => {
                format!("User: {}", s.chars().take(40).collect::<String>())
            }
            langgraph::Message::Assistant(s) => {
                format!("Assistant: {}", s.chars().take(40).collect::<String>())
            }
            _ => "Other".into(),
        };
        println!("  [{}] {}", i + 1, preview);
    }
    if verified.len() > 3 {
        println!("  ... and {} more", verified.len() - 3);
    }
    Ok(())
}
