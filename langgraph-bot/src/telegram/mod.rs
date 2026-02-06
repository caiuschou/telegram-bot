//! Telegram bot integration: handler and stream-edit loop.
//!
//! - **[`handler`]** – `AgentHandler` that runs ReAct agent on reply-to-bot or @mention.
//! - **[`ensure_memory_handler`]** – Ensures bot identity and user profile in long-term memory before the agent.
//! - **[`stream_edit`]** – Consumes `StreamUpdate`s and edits a message in place.

mod stream_edit;

pub mod ensure_memory_handler;
pub mod handler;

pub use ensure_memory_handler::{EnsureLongTermMemoryHandler, EnsureThenAgentHandler};
pub use handler::{AgentHandler, RunnerResolver};
pub use stream_edit::{format_reply_with_process_and_tools, is_message_not_modified_error, run_stream_edit_loop};
