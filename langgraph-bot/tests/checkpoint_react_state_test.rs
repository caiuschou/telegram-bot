//! Unit tests for `langgraph_bot::checkpoint::get_react_state_from_checkpointer`.
//!
//! **BDD style**: Given messages imported into checkpointer, when reading back as ReActState,
//! then messages field matches and tool_calls/tool_results are empty.

use anyhow::Result;
use langgraph::Message;
use langgraph_bot::{
    get_react_state_from_checkpointer, import_messages_into_checkpointer,
    merge_messages_into_checkpointer, verify_messages_integrity,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: Creates a temporary DB path for testing.
fn temp_db_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("test_checkpoint.db");
    (dir, path)
}

/// **Test: get_react_state_from_checkpointer returns default state for non-existent thread.**
#[tokio::test]
async fn get_react_state_nonexistent_thread_returns_default() -> Result<()> {
    let (_dir, db_path) = temp_db_path();
    let state = get_react_state_from_checkpointer(&db_path, "nonexistent").await?;
    assert!(state.messages.is_empty());
    assert!(state.tool_calls.is_empty());
    assert!(state.tool_results.is_empty());
    Ok(())
}

/// **Test: get_react_state_from_checkpointer returns imported messages.**
#[tokio::test]
async fn get_react_state_returns_imported_messages() -> Result<()> {
    let (_dir, db_path) = temp_db_path();
    let original = vec![
        Message::User("Hello".into()),
        Message::Assistant("Hi there".into()),
    ];

    import_messages_into_checkpointer(&db_path, "test_thread", &original).await?;
    let state = get_react_state_from_checkpointer(&db_path, "test_thread").await?;

    verify_messages_integrity(&original, &state.messages)?;
    assert!(state.tool_calls.is_empty());
    assert!(state.tool_results.is_empty());
    Ok(())
}

/// **Test: get_react_state_from_checkpointer handles empty messages.**
#[tokio::test]
async fn get_react_state_empty_messages() -> Result<()> {
    let (_dir, db_path) = temp_db_path();
    let original: Vec<Message> = vec![];

    import_messages_into_checkpointer(&db_path, "empty_thread", &original).await?;
    let state = get_react_state_from_checkpointer(&db_path, "empty_thread").await?;

    assert!(state.messages.is_empty());
    assert!(state.tool_calls.is_empty());
    assert!(state.tool_results.is_empty());
    Ok(())
}

/// **Test: get_react_state_from_checkpointer preserves message order.**
#[tokio::test]
async fn get_react_state_preserves_order() -> Result<()> {
    let (_dir, db_path) = temp_db_path();
    let original = vec![
        Message::User("First".into()),
        Message::Assistant("Second".into()),
        Message::User("Third".into()),
        Message::Assistant("Fourth".into()),
    ];

    import_messages_into_checkpointer(&db_path, "order_thread", &original).await?;
    let state = get_react_state_from_checkpointer(&db_path, "order_thread").await?;

    verify_messages_integrity(&original, &state.messages)?;
    Ok(())
}

/// **Test: Multiple threads maintain separate ReActState.**
#[tokio::test]
async fn get_react_state_separates_threads() -> Result<()> {
    let (_dir, db_path) = temp_db_path();

    let messages_a = vec![Message::User("Thread A".into())];
    let messages_b = vec![Message::User("Thread B".into())];

    import_messages_into_checkpointer(&db_path, "thread_a", &messages_a).await?;
    import_messages_into_checkpointer(&db_path, "thread_b", &messages_b).await?;

    let state_a = get_react_state_from_checkpointer(&db_path, "thread_a").await?;
    let state_b = get_react_state_from_checkpointer(&db_path, "thread_b").await?;

    verify_messages_integrity(&messages_a, &state_a.messages)?;
    verify_messages_integrity(&messages_b, &state_b.messages)?;

    Ok(())
}

/// **Test: merge_messages_into_checkpointer prepends new messages and dedupes by content.**
#[tokio::test]
async fn merge_messages_prepends_and_dedupes() -> Result<()> {
    let (_dir, db_path) = temp_db_path();
    let initial = vec![
        Message::User("Hello".into()),
        Message::Assistant("Hi".into()),
    ];
    import_messages_into_checkpointer(&db_path, "merge_thread", &initial).await?;

    let to_prepend = vec![
        Message::User("Older".into()),
        Message::User("Hello".into()),
        Message::Assistant("Hi".into()),
    ];
    merge_messages_into_checkpointer(&db_path, "merge_thread", &to_prepend).await?;

    let state = get_react_state_from_checkpointer(&db_path, "merge_thread").await?;
    assert_eq!(state.messages.len(), 3, "Older + Hello + Hi (deduped)");
    assert!(matches!(&state.messages[0], Message::User(s) if s == "Older"));
    assert!(matches!(&state.messages[1], Message::User(s) if s == "Hello"));
    assert!(matches!(&state.messages[2], Message::Assistant(s) if s == "Hi"));
    Ok(())
}
