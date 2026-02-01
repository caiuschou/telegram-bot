//! Seed messages into langgraph short-term memory (SqliteSaver) and chat via ReAct agent.
//!
//! **Seed flow**: Load messages → `checkpoint::import_messages_into_checkpointer` (writes `ReActState { messages, tool_calls: [], tool_results: [] }`).
//! **Chat flow**: `react::create_react_runner(db_path)` → `run_chat(&runner, thread_id, content)` loads persistent state, runs Think→Act→Observe, persists, returns reply.

pub mod checkpoint;
pub mod load;
pub mod react;

pub use checkpoint::{
    get_messages_from_checkpointer, get_react_state_from_checkpointer,
    import_messages_into_checkpointer, verify_messages_format, verify_messages_integrity,
};
pub use load::{
    load_messages_from_path, load_messages_from_path_with_stats, load_messages_from_slice,
    load_messages_from_slice_with_stats, seed_messages_to_messages,
    seed_messages_to_messages_with_stats,
};
pub use react::{create_react_runner, ReactRunner};

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
