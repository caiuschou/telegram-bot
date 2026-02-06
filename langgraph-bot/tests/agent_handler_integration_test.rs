//! Integration tests: Message → Handler::handle → Mock Bot receives final edit.
//!
//! Verifies the full path including AgentHandler, queue, RunnerResolver, run_chat_stream,
//! and format_reply without hitting the real Telegram API. Requires OPENAI_API_KEY; skips when unset.

mod common;

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use langgraph_bot::{AgentHandler, RunnerResolver};
use langgraph_bot::{create_react_runner};
use telegram_bot::{Chat, Handler, Message, MessageDirection, User};
use tokio::time::timeout;

use common::mock_bot::{EditRecord, MockBot};

// ---------- Fake message (trigger agent) ----------

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

// ---------- Integration test ----------

/// **Given** OPENAI_API_KEY and a trigger message, **when** we call Handler::handle, **then** the Mock Bot
/// receives at least one edit and the last edit text is non-empty (full reply after stream).
#[tokio::test]
async fn agent_handler_full_chain_final_edit_non_empty() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    }

    let (runner, _, _) = create_react_runner()
        .await
        .expect("create_react_runner");
    let runner = Arc::new(runner);
    let resolver = Arc::new(RunnerResolver::new(runner.clone(), None, None));

    let (mock_bot, mut edit_rx) = MockBot::with_receiver();
    let bot_username = Arc::new(tokio::sync::RwLock::new(None::<String>));
    let bot_user = Arc::new(tokio::sync::RwLock::new(None::<User>));
    let placeholder = "正在思考…".to_string();

    let handler = AgentHandler::new(
        resolver,
        mock_bot,
        bot_username,
        bot_user,
        placeholder,
    );

    let message = fake_message_reply_to_bot(12345, 67890, "Say hello in one short sentence.");
    let _ = handler.handle(&message).await.expect("handle");

    // Wait for edits (streaming + final). Mock channel never closes; collect for up to 90s, last edit = final.
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
