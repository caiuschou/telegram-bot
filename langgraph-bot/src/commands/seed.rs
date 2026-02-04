//! Seed subcommand: generate messages, import into checkpointer, print preview.

use anyhow::Result;
use langgraph_bot::seed_messages_to_messages_with_user_info_with_stats;
use seed_messages::generate_messages;

use super::import_into_checkpointer;
use super::memory::print_import_preview;

/// Seed subcommand: generate messages, import into checkpointer, print preview.
pub async fn cmd_seed(db: &std::path::Path, thread_id: Option<String>) -> Result<()> {
    let (messages, skipped) =
        seed_messages_to_messages_with_user_info_with_stats(generate_messages()?);
    if skipped > 0 {
        eprintln!(
            "Warning: {} messages skipped (direction not received/sent)",
            skipped
        );
    }
    let thread_id = thread_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let id = import_into_checkpointer(db, &thread_id, &messages).await?;
    println!("Seeded thread {} with checkpoint id: {}", thread_id, id);
    print_import_preview(db, &thread_id, &messages).await?;
    Ok(())
}
