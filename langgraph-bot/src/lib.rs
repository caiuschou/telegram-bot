//! Seed messages into langgraph short-term memory (SqliteSaver).
//!
//! **Write flow** (no ReAct):
//! 1. Load: `load::load_messages_from_path("messages.json")` → `Vec<Message>`.
//! 2. Write: `checkpoint::import_messages_into_checkpointer(db_path, thread_id, messages)`.
//!    - Builds `MessagesState { messages }`, `Checkpoint::from_state(..., Input, 0)`, then `checkpointer.put`.
//! 3. Later: use same `thread_id` with a compiled graph + this checkpointer to resume that conversation.

pub mod checkpoint;
pub mod load;

pub use checkpoint::{
    get_messages_from_checkpointer, import_messages_into_checkpointer, verify_messages_format,
    verify_messages_integrity, MessagesState,
};
pub use load::{
    load_messages_from_path, load_messages_from_path_with_stats, load_messages_from_slice,
    load_messages_from_slice_with_stats, seed_messages_to_messages,
    seed_messages_to_messages_with_stats,
};
