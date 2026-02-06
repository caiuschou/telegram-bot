//! Integration tests for `AgentHandler` queue functionality.
//!
//! **BDD style**: Given an AgentHandler with multiple messages,
//! when submitting them concurrently to the same chat, then they are processed serially.
//! When submitting to different chats, then they are processed concurrently.

use std::sync::Arc;
use std::time::Duration;
use telegram_bot::{Bot, Chat, Handler, HandlerResponse, Message, Result, User};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tempfile;

/// Mock Bot that records sent messages and returns mock message IDs.
struct MockBot {
    sender: mpsc::UnboundedSender<(String, String)>,
    delay_ms: u64,
}

impl MockBot {
    fn new(delay_ms: u64) -> (Self, mpsc::UnboundedReceiver<(String, String)>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                sender: tx,
                delay_ms,
            },
            rx,
        )
    }
}

#[async_trait::async_trait]
impl Bot for MockBot {
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()> {
        sleep(Duration::from_millis(self.delay_ms)).await;
        let _ = self.sender.send((chat.id.to_string(), text.to_string()));
        Ok(())
    }

    async fn reply_to(&self, message: &Message, text: &str) -> Result<()> {
        self.send_message(&message.chat, text).await
    }

    async fn send_message_and_return_id(&self, chat: &Chat, text: &str) -> Result<String> {
        sleep(Duration::from_millis(self.delay_ms)).await;
        let _ = self.sender.send((chat.id.to_string(), text.to_string()));
        Ok("mock_msg_id_1".to_string())
    }

    async fn edit_message(&self, chat: &Chat, _message_id: &str, text: &str) -> Result<()> {
        sleep(Duration::from_millis(10)).await;
        let _ = self.sender.send((chat.id.to_string(), format!("[EDIT] {}", text)));
        Ok(())
    }
}

/// Creates a test message with the given chat_id and content.
fn create_test_message(chat_id: i64, user_id: i64, content: &str) -> Message {
    Message {
        id: format!("msg_{}", user_id),
        user: User {
            id: user_id,
            username: Some(format!("user_{}", user_id)),
            first_name: Some(format!("User{}", user_id)),
            last_name: None,
        },
        chat: Chat {
            id: chat_id,
            chat_type: "private".to_string(),
        },
        content: content.to_string(),
        message_type: "text".to_string(),
        direction: telegram_bot::MessageDirection::Incoming,
        created_at: chrono::Utc::now(),
        reply_to_message_id: None,
        reply_to_message_from_bot: false,
        reply_to_message_content: None,
    }
}

/// **Test: Handler returns Continue for non-trigger messages.**
#[tokio::test]
async fn non_trigger_returns_continue() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    }

    let (mock_bot, _rx) = MockBot::new(10);
    let mock_bot = Arc::new(mock_bot);

    let bot_username = Arc::new(tokio::sync::RwLock::new(Some("test_bot".to_string())));

    let (runner, _, _) = langgraph_bot::create_react_runner().await.unwrap();
    let runner = Arc::new(runner);

    let handler = Arc::new(langgraph_bot::AgentHandler::new(
        runner,
        mock_bot.clone() as Arc<dyn Bot>,
        bot_username,
        "正在思考…".to_string(),
    ));

    let msg = create_test_message(12345, 1, "Just a regular message without @mention or reply");
    let result = handler.handle(&msg).await;

    assert!(result.is_ok());
    match result.unwrap() {
        HandlerResponse::Continue => {}
        _ => panic!("Expected Continue for non-trigger message"),
    }
}

/// **Test: @mention triggers processing.**
#[tokio::test]
async fn mention_triggers_processing() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    }

    let (mock_bot, mut rx) = MockBot::new(10);
    let mock_bot = Arc::new(mock_bot);

    let bot_username = Arc::new(tokio::sync::RwLock::new(Some("test_bot".to_string())));

    let (runner, _, _) = langgraph_bot::create_react_runner().await.unwrap();
    let runner = Arc::new(runner);

    let handler = Arc::new(langgraph_bot::AgentHandler::new(
        runner,
        mock_bot.clone() as Arc<dyn Bot>,
        bot_username,
        "正在思考…".to_string(),
    ));

    let msg = create_test_message(12345, 1, "@test_bot hello");
    let _ = handler.handle(&msg).await;

    let timeout = tokio::time::sleep(Duration::from_secs(1));
    tokio::pin!(timeout);

    let mut received = false;
    tokio::select! {
        _ = &mut timeout => {}
        Some((_chat, text)) = rx.recv() => {
            if text.contains("正在思考") {
                received = true;
            }
        }
    }

    assert!(received, "Expected placeholder message for @mention");
}

/// **Test: Multiple @mention messages to same chat are queued.**
#[tokio::test]
async fn multiple_mentions_same_chat_queued() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("Skipping test: OPENAI_API_KEY not set");
        return;
    }

    let (mock_bot, mut rx) = MockBot::new(50);
    let mock_bot = Arc::new(mock_bot);

    let bot_username = Arc::new(tokio::sync::RwLock::new(Some("test_bot".to_string())));

    let (runner, _, _) = langgraph_bot::create_react_runner().await.unwrap();
    let runner = Arc::new(runner);

    let handler = Arc::new(langgraph_bot::AgentHandler::new(
        runner,
        mock_bot.clone() as Arc<dyn Bot>,
        bot_username,
        "正在思考…".to_string(),
    ));

    let chat_id = 12345;
    let num_messages: usize = 3;

    for i in 1..=num_messages as i64 {
        let msg = create_test_message(chat_id, i, &format!("@test_bot question_{}", i));
        let result = handler.handle(&msg).await;
        assert!(result.is_ok());
        match result.unwrap() {
            HandlerResponse::Continue => {}
            _ => panic!("Expected Continue for queued messages"),
        }
    }

    let mut received_count: usize = 0;
    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    while received_count < num_messages {
        tokio::select! {
            _ = &mut timeout => {
                panic!("Timeout: expected {} messages, got {}", num_messages, received_count);
            }
            Some((_chat, text)) = rx.recv() => {
                if text.contains("正在思考") {
                    received_count += 1;
                }
            }
        }
    }

    assert_eq!(received_count, num_messages);
}

#[allow(dead_code)]
fn temp_db_path() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let path = dir.path().join("test_agent_handler.db");
    (dir, path)
}
