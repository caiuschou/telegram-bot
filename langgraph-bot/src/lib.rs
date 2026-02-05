//! Seed messages into langgraph short-term memory (SqliteSaver) and chat via ReAct agent.
//!
//! **Seed flow**: Load messages → `checkpoint::import_messages_into_checkpointer` (writes `ReActState { messages, tool_calls: [], tool_results: [] }`).
//! **Chat flow**: `react::create_react_runner(db_path)` → `run_chat_stream(&runner, thread_id, content, on_chunk)` loads persistent state, runs Think→Act→Observe, streams chunks, persists, returns final reply.

pub mod checkpoint;
pub mod format;
pub mod load;
pub mod llm_request_logging;
pub mod react;
pub mod telegram_db;

pub mod telegram_handler;
mod run;

pub use run::run_telegram;
pub use checkpoint::{
    append_user_message_into_checkpointer, format_thread_summary, get_messages_from_checkpointer,
    get_react_state_from_checkpointer, import_messages_into_checkpointer, list_thread_ids,
    merge_messages_into_checkpointer, verify_messages_format, verify_messages_integrity,
};
pub use format::user_info_prefix;
pub use load::{
    load_messages_from_path, load_messages_from_path_with_stats,
    load_messages_from_path_with_user_info, load_messages_from_path_with_user_info_with_stats,
    load_messages_from_slice, load_messages_from_slice_with_stats,
    load_messages_from_slice_with_user_info, load_messages_from_slice_with_user_info_with_stats,
    seed_messages_to_messages, seed_messages_to_messages_with_stats,
    seed_messages_to_messages_with_user_info, seed_messages_to_messages_with_user_info_with_stats,
};
pub use react::{
    create_react_runner, last_assistant_content, print_runtime_info, ChatStreamResult, ReactRunner,
    StreamUpdate, UserProfile,
};
pub use telegram_db::{load_all_messages_from_telegram_db, load_messages_from_telegram_db};

pub use telegram_handler::AgentHandler;

/// Runs one chat turn with streaming: `on_update` is called for each chunk and when steps/tools change; returns final result.
///
/// When `user_message_already_in_checkpoint` is true, the user message is not appended again (caller has already written it to short-term memory, e.g. via `append_user_message_into_checkpointer`).
pub async fn run_chat_stream(
    runner: &ReactRunner,
    thread_id: &str,
    content: &str,
    on_update: impl FnMut(StreamUpdate) + Send,
    user_profile: Option<&UserProfile>,
    user_message_already_in_checkpoint: bool,
) -> anyhow::Result<ChatStreamResult> {
    runner
        .run_chat_stream(
            thread_id,
            content,
            on_update,
            user_profile,
            user_message_already_in_checkpoint,
        )
        .await
}
