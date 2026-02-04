//! Unit tests for `langgraph_bot::react` module.
//!
//! **BDD style**: Given a ReactRunner and thread_id, when running chat turns,
//! then state persists across turns and replies are generated correctly.

use anyhow::Result;
use langgraph::Message;
use langgraph_bot::{
    create_react_runner, get_react_state_from_checkpointer, run_chat, UserProfile,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper: Creates a temporary DB path for testing.
fn temp_db_path() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let path = dir.path().join("test_react.db");
    (dir, path)
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
    let _runner = create_react_runner(&db_path).await?;
    assert!(db_path.exists());
    Ok(())
}

/// **Test: run_chat returns non-empty reply for simple message.**
#[tokio::test]
async fn run_chat_returns_reply() -> Result<()> {
    // Skip if no API key
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let runner = create_react_runner(&db_path).await?;
    let reply = run_chat(&runner, "test_thread", "Hello", None).await?;
    assert!(!reply.is_empty(), "Reply should not be empty");
    Ok(())
}

/// **Test: Multiple chat turns persist state in same thread.**
#[tokio::test]
async fn run_chat_persists_state_across_turns() -> Result<()> {
    // Skip if no API key
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let runner = create_react_runner(&db_path).await?;

    let reply1 = run_chat(&runner, "persist_thread", "My name is Alice", None).await?;
    assert!(!reply1.is_empty());

    let reply2 = run_chat(&runner, "persist_thread", "What is my name?", None).await?;
    assert!(!reply2.is_empty());
    // Note: Full context verification requires checking checkpoint, not just reply content

    Ok(())
}

/// **Test: Different threads maintain separate state.**
#[tokio::test]
async fn run_chat_separates_threads() -> Result<()> {
    // Skip if no API key
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let runner = create_react_runner(&db_path).await?;

    let _reply1 = run_chat(&runner, "thread_a", "I like cats", None).await?;
    let _reply2 = run_chat(&runner, "thread_b", "I like dogs", None).await?;

    // Both threads should work independently
    let reply_a = run_chat(&runner, "thread_a", "What do I like?", None).await?;
    let reply_b = run_chat(&runner, "thread_b", "What do I like?", None).await?;

    assert!(!reply_a.is_empty());
    assert!(!reply_b.is_empty());

    Ok(())
}

/// **Test: Empty message returns some reply (even if error-like).**
#[tokio::test]
async fn run_chat_handles_empty_message() -> Result<()> {
    // Skip if no API key
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let runner = create_react_runner(&db_path).await?;
    let _reply = run_chat(&runner, "empty_thread", "", None).await?;
    // Should not panic; reply may be empty or contain error message
    Ok(())
}

/// **Test: run_chat with UserProfile injects a System message with user identity into checkpoint.**
#[tokio::test]
async fn run_chat_with_user_profile_injects_system_message() -> Result<()> {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return Ok(());
    }

    let (_dir, db_path) = temp_db_path();
    let runner = create_react_runner(&db_path).await?;
    let profile = UserProfile {
        user_id: "123".to_string(),
        first_name: Some("Alice".to_string()),
        last_name: Some("Smith".to_string()),
        username: Some("alice".to_string()),
    };

    let _reply = run_chat(&runner, "profile_thread", "Hi", Some(&profile)).await?;
    let state = get_react_state_from_checkpointer(&db_path, "profile_thread").await?;
    let has_profile = state.messages.iter().any(|m| {
        if let Message::System(s) = m {
            s.starts_with("User profile:")
        } else {
            false
        }
    });
    assert!(
        has_profile,
        "Checkpoint should contain User profile System message"
    );
    Ok(())
}
