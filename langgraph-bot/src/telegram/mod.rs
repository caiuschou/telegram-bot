//! Telegram bot integration: handler and stream-edit loop.
//!
//! - **[`handler`]** – `AgentHandler` that runs ReAct agent on reply-to-bot or @mention.
//! - **[`stream_edit`]** – Consumes `StreamUpdate`s and edits a message in place.

mod stream_edit;

pub mod handler;

pub use handler::AgentHandler;
pub use stream_edit::{format_reply_with_process_and_tools, is_message_not_modified_error, run_stream_edit_loop};
