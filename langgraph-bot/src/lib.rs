//! Seed messages into langgraph short-term memory (SqliteSaver) and chat via ReAct agent.
//!
//! **Seed flow**: Load messages → `checkpoint::import_messages_into_checkpointer` (writes `ReActState { messages, tool_calls: [], tool_results: [] }`).
//! **Chat flow**: `react::create_react_runner(db_path)` → `run_chat(&runner, thread_id, content)` loads persistent state, runs Think→Act→Observe, persists, returns reply.

pub mod checkpoint;
pub mod format;
pub mod load;
pub mod react;

pub use checkpoint::{
    format_thread_summary, get_messages_from_checkpointer, get_react_state_from_checkpointer,
    import_messages_into_checkpointer, list_thread_ids, verify_messages_format,
    verify_messages_integrity,
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
pub use react::{create_react_runner, print_runtime_info, ReactRunner};

/// Runs one chat turn using the given runner (loads persistent state, appends user message, invokes ReAct, returns reply).
///
/// **Interaction**: Used by CLI `chat` and interactive loop. Create a runner once with `create_react_runner(db_path)`.
pub async fn run_chat(
    runner: &ReactRunner,
    thread_id: &str,
    content: &str,
) -> anyhow::Result<String> {
    runner.run_chat(thread_id, content).await
}

/// Runs one chat turn with streaming: `on_chunk` is called for each LLM token; returns final reply.
pub async fn run_chat_stream(
    runner: &ReactRunner,
    thread_id: &str,
    content: &str,
    on_chunk: impl FnMut(&str) + Send,
) -> anyhow::Result<String> {
    runner.run_chat_stream(thread_id, content, on_chunk).await
}
