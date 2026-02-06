//! Integration tests: real config + run_bot_with_custom_handler build path + full memory/embedding + run_telegram handler chain.
//!
//! Exercises load_config, create_memory_stores_for_llm, build_bot_components, make_handler (build_run_telegram_handler),
//! then drives the handler chain with a fake message and asserts the Mock Bot receives a non-empty final edit.
//! Complements `agent_handler_integration_test`, which tests only AgentHandler without config or build-only path.

mod common;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use langgraph_bot::{build_run_telegram_handler, create_react_runner};
use telegram_bot::{load_config, Chat, Message, MessageDirection, User};
use telegram_llm_bot::run_bot_with_custom_handler_build_only;
use tokio::time::timeout;

use common::mock_bot::{EditRecord, MockBot};

/// Override env so DB and memory use in-memory storage; avoids "unable to open database file" when .env paths are relative/invalid under `cargo test` cwd.
/// Uses shared in-memory SQLite so the connection pool shares one DB (otherwise each connection gets its own empty :memory: and "no such table" occurs).
fn set_test_env_overrides() {
    std::env::set_var("DATABASE_URL", "file::memory:?cache=shared");
    std::env::set_var("MEMORY_STORE_TYPE", "memory");
}

/// Builds a message that triggers the agent via reply-to-bot: `get_question` returns `Some(content)`.
fn fake_message_reply_to_bot(chat_id: i64, user_id: i64, content: &str) -> Message {
    Message {
        id: "msg-1".to_string(),
        user: User {
            id: user_id,
            username: Some("testuser".to_string()),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
        },
        chat: Chat {
            id: chat_id,
            chat_type: "private".to_string(),
        },
        content: content.to_string(),
        message_type: "text".to_string(),
        direction: MessageDirection::Incoming,
        created_at: Utc::now(),
        reply_to_message_id: Some("bot-msg-1".to_string()),
        reply_to_message_from_bot: true,
        reply_to_message_content: None,
    }
}

/// **Given** OPENAI_API_KEY and BOT_TOKEN, **when** we build via run_bot_with_custom_handler_build_only (with Mock Bot)
/// and call handler_chain.handle(&message), **then** the Mock Bot receives at least one edit and the last edit text is non-empty.
///
/// Loads `.env` from the current working directory (usually workspace root when running `cargo test`) so that
/// BOT_TOKEN, OPENAI_API_KEY, and other config are available without exporting them in the shell.
/// Overrides DATABASE_URL to `:memory:` (in-memory SQLite) and MEMORY_STORE_TYPE to `memory` so the test does not depend on .env paths.
#[tokio::test]
async fn run_telegram_handler_chain_final_edit_non_empty() {
    let _ = dotenvy::dotenv();
    set_test_env_overrides();

    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set (set in .env or environment)");
        return;
    }
    let config = match load_config(None) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping test: load_config failed (e.g. BOT_TOKEN not set): {}", e);
            return;
        }
    };

    let (runner, _, _) = create_react_runner()
        .await
        .expect("create_react_runner");
    let runner = Arc::new(runner);
    let placeholder = "正在思考…".to_string();

    let (mock_bot, mut edit_rx) = MockBot::with_receiver();

    let handler_chain = run_bot_with_custom_handler_build_only(
        config,
        Some(mock_bot),
        move |_config, components| {
            build_run_telegram_handler(
                runner.clone(),
                components
                    .handler_bot
                    .clone()
                    .expect("handler_bot injected"),
                components.bot_username.clone(),
                components.bot_user.clone(),
                placeholder.clone(),
                None, // simplify: RunnerResolver with None, None
                None,
            )
        },
    )
    .await
    .expect("run_bot_with_custom_handler_build_only");

    let message = fake_message_reply_to_bot(12345, 67890, "Say hello in one short sentence.");
    let _ = handler_chain.handle(&message).await.expect("handle");

    let mut edits: Vec<EditRecord> = Vec::new();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(90) {
        match timeout(Duration::from_secs(1), edit_rx.recv()).await {
            Ok(Some(edit)) => edits.push(edit),
            Ok(None) => break,
            Err(_) => {}
        }
    }

    let final_edit = edits
        .last()
        .expect("at least one edit_message call from process_message");
    assert!(
        !final_edit.text.trim().is_empty(),
        "final edit text should be non-empty, got: {:?}",
        final_edit.text
    );
}
