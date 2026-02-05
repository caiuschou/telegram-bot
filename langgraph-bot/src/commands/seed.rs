//! Seed subcommand: generate messages, print preview. Does not import into checkpointer.

use anyhow::Result;
use langgraph_bot::seed_messages_to_messages_with_user_info_with_stats;
use seed_messages::generate_messages;

use super::memory::print_messages_preview;

/// Seed subcommand: generate messages, print preview. Does not import into checkpointer.
pub fn cmd_seed(_db: &std::path::Path, _thread_id: Option<String>) -> Result<()> {
    let (messages, skipped) =
        seed_messages_to_messages_with_user_info_with_stats(generate_messages()?);
    if skipped > 0 {
        eprintln!(
            "Warning: {} messages skipped (direction not received/sent)",
            skipped
        );
    }
    println!("Generated {} messages (not imported into checkpoint)", messages.len());
    print_messages_preview(&messages, 3);
    Ok(())
}
