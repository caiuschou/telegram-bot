//! ReAct agent CLI and Telegram bot: chat, run, load/seed message preview.
//!
//! **Short-term memory**: Disabled; each turn uses only the current message (NoOp checkpointer).
//! **Chat flow**: `react::create_react_runner()` → `run_chat_stream(&runner, thread_id, content, on_chunk)` builds fresh state per turn, runs Think→Act→Observe, streams chunks, returns final reply.
//! **Optional checkpoint** (offline/test): `checkpoint::import_messages_into_checkpointer` for seeding; not used by Run/Chat.

pub mod checkpoint;
pub mod format;
pub mod load;
pub mod llm_request_logging;
pub mod memory;
pub mod noop_checkpointer;
pub mod react;
pub mod telegram_db;
pub mod tools;

pub mod telegram;
mod run;

pub use run::{build_run_telegram_handler, run_telegram};
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
pub use memory::LongTermMemoryPolicy;
pub use react::{
    create_react_runner, last_assistant_content, print_runtime_info, ChatStreamResult, ReactRunner,
    StreamUpdate, UserProfile,
};
pub use telegram_db::{load_all_messages_from_telegram_db, load_messages_from_telegram_db};

pub use telegram::{AgentHandler, RunnerResolver};

/// Runs one chat turn with streaming: `on_update` is called for each chunk and when steps/tools change; returns final result.
///
/// When `user_message_already_in_checkpoint` is true, the user message is not appended again (caller has already put it in state). With short-term memory disabled, callers typically pass `false` and the message content so each turn gets fresh state.
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
