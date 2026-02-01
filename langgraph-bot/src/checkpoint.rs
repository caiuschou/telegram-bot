//! Write and read messages in langgraph short-term memory (Checkpointer / SqliteSaver).
//!
//! **Write flow**: Load messages → build state → `Checkpoint::from_state` → `checkpointer.put`.
//! **Verification**: After put, `get_messages_from_checkpointer` reads back the latest checkpoint for the thread.

use anyhow::Result;
use langgraph::memory::{
    Checkpoint, CheckpointSource, Checkpointer, JsonSerializer, RunnableConfig, SqliteSaver,
};
use std::path::Path;
use std::sync::Arc;

/// State type for short-term memory: only messages (no ReAct tool_calls/tool_results).
///
/// Used by `Checkpoint::from_state` and by `Checkpointer::put` / `get_tuple`; the checkpointer
/// stores and returns this type in `Checkpoint::channel_values`.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MessagesState {
    pub messages: Vec<langgraph::Message>,
}

/// Builds a shared `SqliteSaver` checkpointer for `MessagesState` at the given DB path.
/// Used by `import_messages_into_checkpointer` and `get_messages_from_checkpointer`.
fn make_checkpointer(db_path: impl AsRef<Path>) -> Result<Arc<dyn Checkpointer<MessagesState>>> {
    let serializer = Arc::new(JsonSerializer);
    let checkpointer: Arc<dyn Checkpointer<MessagesState>> = Arc::new(
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
/// as the initial checkpoint for `thread_id`. Later runs with the same `thread_id`
/// will see this conversation as short-term memory. Use `get_messages_from_checkpointer`
/// to read back and verify after seeding.
///
/// **Write flow**:
/// 1. Build `MessagesState { messages }`.
/// 2. Build `Checkpoint::from_state(state, CheckpointSource::Input, 0)`.
/// 3. `config = RunnableConfig { thread_id: Some(thread_id), .. }`.
/// 4. `checkpointer.put(&config, &checkpoint).await`.
pub async fn import_messages_into_checkpointer(
    db_path: impl AsRef<Path>,
    thread_id: &str,
    messages: &[langgraph::Message],
) -> Result<String> {
    let checkpointer = make_checkpointer(db_path)?;
    let state = MessagesState {
        messages: messages.to_vec(),
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
    let checkpointer = make_checkpointer(db_path)?;
    let config = make_config(thread_id);
    let tuple = checkpointer
        .get_tuple(&config)
        .await
        .map_err(|e| anyhow::anyhow!("checkpoint get_tuple: {}", e))?;
    Ok(tuple
        .map(|(cp, _)| cp.channel_values.messages)
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
