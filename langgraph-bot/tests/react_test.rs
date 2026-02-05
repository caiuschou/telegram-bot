//! Unit tests for `langgraph_bot::react` module.
//!
//! **BDD style**: Given a ReactRunner and thread_id, when running chat turns,
//! then state persists across turns and replies are generated correctly.

use anyhow::Result;
use langgraph::Message;
use langgraph::ReActState;
use langgraph_bot::{
    create_react_runner, get_react_state_from_checkpointer, last_assistant_content,
    run_chat_stream,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: Creates a temporary DB path for testing.
fn temp_db_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("test_react.db");
    (dir, path)
}

/// **Test: create_react_runner without OPENAI_API_KEY returns a clear error.**
#[tokio::test]
async fn create_react_runner_fails_without_api_key() -> Result<()> {
    let (_dir, db_path) = temp_db_path();
    let key = std::env::var("OPENAI_API_KEY").ok();
    std::env::remove_var("OPENAI_API_KEY");
    let result = create_react_runner(&db_path).await;
    if let Some(k) = key {
        std::env::set_var("OPENAI_API_KEY", k);
    }
    match result {
        Ok(_) => panic!("create_react_runner should fail without OPENAI_API_KEY"),
        Err(e) => assert!(
            e.to_string().contains("OPENAI_API_KEY"),
            "error message should mention OPENAI_API_KEY, got: {}",
            e
        ),
    }
    Ok(())
}

/// **Test: ReactRunner can be created with valid DB path.**
#[tokio::test]
async fn create_react_runner_succeeds() -> Result<()> {
    // Skip if no API key
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let (_runner, _, _) = create_react_runner(&db_path).await?;
    assert!(db_path.exists());
    Ok(())
}

// --- run_chat_stream and last_assistant_content (plan §6 stage 2) ---

/// **Test: last_assistant_content returns last Assistant message; empty when no Assistant.**
#[test]
fn last_assistant_content_helper() {
    // No Assistant -> empty string
    let state = ReActState {
        messages: vec![Message::system("s"), Message::user("u")],
        tool_calls: vec![],
        tool_results: vec![],
        turn_count: 0,
    };
    assert_eq!(last_assistant_content(&state), "");

    // Last is Assistant -> its content
    let state = ReActState {
        messages: vec![
            Message::system("s"),
            Message::user("u1"),
            Message::Assistant("a".to_string()),
            Message::user("u2"),
            Message::Assistant("b".to_string()),
        ],
        tool_calls: vec![],
        tool_results: vec![],
        turn_count: 0,
    };
    assert_eq!(last_assistant_content(&state), "b");
}

/// **Test: run_chat_stream invokes on_chunk and returns non-empty final reply.**
#[tokio::test]
async fn run_chat_stream_invokes_on_chunk() -> Result<()> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let (runner, _, _) = create_react_runner(&db_path).await?;
    let mut chunks: Vec<String> = vec![];
    let result = run_chat_stream(
        &runner,
        "stream_thread",
        "Say hello in one word",
        |update| {
            match update {
                langgraph_bot::StreamUpdate::Chunk(s) | langgraph_bot::StreamUpdate::ThinkChunk(s) => {
                    chunks.push(s);
                }
                _ => {}
            }
        },
        None,
        false,
    )
    .await?;

    assert!(!result.reply.is_empty(), "stream should return non-empty reply");
    assert!(!chunks.is_empty(), "on_chunk should be called at least once");
    Ok(())
}

/// **Test: run_chat_stream return value equals last assistant content in checkpoint.**
#[tokio::test]
async fn run_chat_stream_returns_last_assistant_content() -> Result<()> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let (runner, _, _) = create_react_runner(&db_path).await?;
    let result = run_chat_stream(
        &runner,
        "stream_final_thread",
        "Reply with exactly: ok",
        |_| {},
        None,
        false,
    )
    .await?;

    let state = get_react_state_from_checkpointer(&db_path, "stream_final_thread").await?;
    let from_state = last_assistant_content(&state);
    assert_eq!(
        result.reply, from_state,
        "run_chat_stream return should match last assistant content in checkpoint"
    );
    Ok(())
}

// --- print_runtime_info (plan §6 stage 3) ---

/// **Test: print_runtime_info prints configuration without errors.**
#[tokio::test]
async fn print_runtime_info_prints_config() -> Result<()> {
    let (_dir, db_path) = temp_db_path();

    let original_model = std::env::var("OPENAI_MODEL").ok();
    std::env::set_var("OPENAI_MODEL", "test-model");

    let result = langgraph_bot::print_runtime_info(&db_path).await;

    if let Some(m) = original_model {
        std::env::set_var("OPENAI_MODEL", m);
    } else {
        std::env::remove_var("OPENAI_MODEL");
    }

    assert!(
        result.is_ok(),
        "print_runtime_info should succeed: {:?}",
        result
    );
    Ok(())
}
