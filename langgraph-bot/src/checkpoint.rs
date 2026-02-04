//! Write and read messages in langgraph short-term memory (Checkpointer / SqliteSaver).
//!
//! Uses `langgraph::ReActState` so the same checkpointer can be used by the ReAct graph.
//! **Write flow**: Load messages → build `ReActState { messages, tool_calls: [], tool_results: [] }` → put.
//! **Verification**: After put, `get_messages_from_checkpointer` reads back `.messages` from the latest checkpoint.

use anyhow::Result;
use langgraph::Message;
use langgraph::memory::{
    Checkpoint, CheckpointSource, Checkpointer, JsonSerializer, RunnableConfig, SqliteSaver,
};
use langgraph::ReActState;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;

/// Builds a shared `SqliteSaver` checkpointer for `ReActState` at the given DB path.
/// Used by `import_messages_into_checkpointer`, `get_messages_from_checkpointer`, and `get_react_state_from_checkpointer`.
fn make_checkpointer(db_path: impl AsRef<Path>) -> Result<Arc<dyn Checkpointer<ReActState>>> {
    let serializer = Arc::new(JsonSerializer);
    let checkpointer: Arc<dyn Checkpointer<ReActState>> = Arc::new(
        SqliteSaver::new(db_path.as_ref(), serializer)
            .map_err(|e| anyhow::anyhow!("SqliteSaver at {:?}: {}", db_path.as_ref(), e))?,
    );
    Ok(checkpointer)
}

/// Builds a `RunnableConfig` with the given `thread_id` for checkpoint put/get.
/// Used by `import_messages_into_checkpointer` and `get_messages_from_checkpointer`.
fn make_config(thread_id: &str) -> RunnableConfig {
    RunnableConfig {
        thread_id: Some(thread_id.to_string()),
        checkpoint_id: None,
        checkpoint_ns: String::new(),
        user_id: None,
    }
}

/// Creates a persistent checkpointer (SqliteSaver) and writes the given messages
/// as the initial checkpoint for `thread_id`. Uses `ReActState { messages, tool_calls: [], tool_results: [] }`
/// so the same DB can be used by the ReAct graph. Use `get_messages_from_checkpointer` to read back after seeding.
///
/// **Write flow**:
/// 1. Build `ReActState { messages, tool_calls: [], tool_results: [] }`.
/// 2. Build `Checkpoint::from_state(state, CheckpointSource::Input, 0)`.
/// 3. `config = RunnableConfig { thread_id: Some(thread_id), .. }`.
/// 4. `checkpointer.put(&config, &checkpoint).await`.
pub async fn import_messages_into_checkpointer(
    db_path: impl AsRef<Path>,
    thread_id: &str,
    messages: &[langgraph::Message],
) -> Result<String> {
    let checkpointer = make_checkpointer(db_path)?;
    let state = ReActState {
        messages: messages.to_vec(),
        tool_calls: vec![],
        tool_results: vec![],
        turn_count: 0,
    };
    let checkpoint = Checkpoint::from_state(state, CheckpointSource::Input, 0);
    let config = make_config(thread_id);

    let id = checkpointer
        .put(&config, &checkpoint)
        .await
        .map_err(|e| anyhow::anyhow!("checkpoint put: {}", e))?;
    Ok(id)
}

/// Reads the latest checkpoint for `thread_id` from the SqliteSaver and returns its messages.
/// Used to verify after seed. Returns empty vec if no checkpoint exists.
pub async fn get_messages_from_checkpointer(
    db_path: impl AsRef<Path>,
    thread_id: &str,
) -> Result<Vec<langgraph::Message>> {
    let state = get_react_state_from_checkpointer(db_path, thread_id).await?;
    Ok(state.messages)
}

/// Reads the latest checkpoint for `thread_id` and returns the full `ReActState`.
/// Used by the ReAct chat flow to load persistent memory before invoke. Returns `Default::default()` if no checkpoint.
pub async fn get_react_state_from_checkpointer(
    db_path: impl AsRef<Path>,
    thread_id: &str,
) -> Result<ReActState> {
    let checkpointer = make_checkpointer(db_path)?;
    let config = make_config(thread_id);
    let tuple = checkpointer
        .get_tuple(&config)
        .await
        .map_err(|e| anyhow::anyhow!("checkpoint get_tuple: {}", e))?;
    Ok(tuple
        .map(|(cp, _)| cp.channel_values)
        .unwrap_or_default())
}

/// Checks that read-back messages match original: same length, same variant (User/Assistant/System), same content per index.
/// Use after get_messages_from_checkpointer to verify integrity.
pub fn verify_messages_integrity(
    original: &[langgraph::Message],
    read_back: &[langgraph::Message],
) -> Result<()> {
    if original.len() != read_back.len() {
        anyhow::bail!(
            "integrity: length mismatch (expected {}, got {})",
            original.len(),
            read_back.len()
        );
    }
    for (i, (a, b)) in original.iter().zip(read_back.iter()).enumerate() {
        match (a, b) {
            (langgraph::Message::System(s1), langgraph::Message::System(s2)) if s1 == s2 => {}
            (langgraph::Message::User(u1), langgraph::Message::User(u2)) if u1 == u2 => {}
            (langgraph::Message::Assistant(a1), langgraph::Message::Assistant(a2)) if a1 == a2 => {}
            _ => {
                anyhow::bail!(
                    "integrity: message[{}] mismatch: {:?} vs {:?}",
                    i,
                    a,
                    b
                );
            }
        }
    }
    Ok(())
}

/// Lists all thread_ids that have at least one checkpoint in the given DB.
/// Reads directly from the `checkpoints` table (same schema as langgraph SqliteSaver).
/// Used by the Memory summary command to show all threads when no thread_id is specified.
pub fn list_thread_ids(db_path: impl AsRef<Path>) -> Result<Vec<String>> {
    let conn = Connection::open(db_path.as_ref())
        .map_err(|e| anyhow::anyhow!("open checkpoint db: {}", e))?;
    let mut stmt = conn.prepare("SELECT DISTINCT thread_id FROM checkpoints ORDER BY thread_id")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut ids: Vec<String> = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

/// Returns a one-line preview of a message for summary display (e.g. "User: [User: Alice / @alice] 在吗？" truncated).
fn message_preview(msg: &Message, max_len: usize) -> String {
    let (role, s) = match msg {
        Message::User(s) => ("User", s.as_str()),
        Message::Assistant(s) => ("Assistant", s.as_str()),
        Message::System(s) => ("System", s.as_str()),
    };
    let content: String = s.chars().take(max_len).collect();
    if content.len() < s.chars().count() {
        format!("{}: {}...", role, content)
    } else {
        format!("{}: {}", role, content)
    }
}

/// Builds a short summary of ReActState for one thread: message count, turn_count, first and last message previews.
/// Used by the Memory summary command. Interacts with `get_react_state_from_checkpointer` for state; this only formats.
pub fn format_thread_summary(
    thread_id: &str,
    state: &ReActState,
    preview_len: usize,
) -> String {
    let n = state.messages.len();
    let first = state
        .messages
        .first()
        .map(|m| message_preview(m, preview_len))
        .unwrap_or_else(|| "—".to_string());
    let last = state
        .messages
        .last()
        .map(|m| message_preview(m, preview_len))
        .unwrap_or_else(|| "—".to_string());
    format!(
        "  thread_id: {}\n  messages: {}  turn_count: {}\n  first: {}\n  last:  {}",
        thread_id, n, state.turn_count, first, last
    )
}

/// Checks that each message is User or Assistant (expected shape for seeded data).
/// Fails if any message is System.
pub fn verify_messages_format(messages: &[langgraph::Message]) -> Result<()> {
    for (i, msg) in messages.iter().enumerate() {
        match msg {
            langgraph::Message::User(_) | langgraph::Message::Assistant(_) => {}
            langgraph::Message::System(_) => {
                anyhow::bail!("format: message[{}] is System (expected only User/Assistant)", i);
            }
        }
    }
    Ok(())
}
