//! CLI command handlers: load, seed, memory (and shared helpers).
//!
//! Dispatched from `main.rs` based on `cli::Commands`.

mod load;
mod memory;
mod seed;

pub use load::cmd_load;
pub use memory::print_memory_summary;
pub use seed::cmd_seed;

use anyhow::Result;
use langgraph_bot::import_messages_into_checkpointer;

/// Imports messages into the checkpointer and returns the checkpoint id.
pub(super) async fn import_into_checkpointer(
    db: &std::path::Path,
    thread_id: &str,
    messages: &[langgraph::Message],
) -> Result<String> {
    import_messages_into_checkpointer(db, thread_id, messages).await
}
