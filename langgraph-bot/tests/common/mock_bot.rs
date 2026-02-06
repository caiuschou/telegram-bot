//! Mock implementation of [`telegram_bot::Bot`] for integration tests.
//!
//! Records `edit_message` and `send_message_and_return_id` calls so tests can
//! wait for the final edit and assert on the reply text without hitting Telegram.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use telegram_bot::{Bot, Chat, Message, Result};

/// One recorded call to `edit_message(chat, message_id, text)`.
#[derive(Debug, Clone)]
#[allow(dead_code)] // chat_id, message_id kept for future assertions
pub struct EditRecord {
    pub chat_id: i64,
    pub message_id: String,
    pub text: String,
}

/// Mock Bot that records edits and returns a fixed placeholder message id.
/// Tests can take the receiver and wait for `EditRecord`s to assert on the final text.
pub struct MockBot {
    /// Fixed id returned by `send_message_and_return_id`.
    placeholder_id: String,
    /// Sender for every `edit_message` call; receiver is held by the test.
    edit_tx: mpsc::UnboundedSender<EditRecord>,
}

impl MockBot {
    /// Creates a MockBot that returns `placeholder_id` from `send_message_and_return_id`
    /// and sends each `edit_message(chat, message_id, text)` as `EditRecord` to `edit_tx`.
    pub fn new(placeholder_id: String, edit_tx: mpsc::UnboundedSender<EditRecord>) -> Self {
        Self {
            placeholder_id,
            edit_tx,
        }
    }

    /// Creates a MockBot and returns the receiver for edit records.
    /// Default placeholder id is `"1"`.
    pub fn with_receiver() -> (Arc<Self>, mpsc::UnboundedReceiver<EditRecord>) {
        let (edit_tx, edit_rx) = mpsc::unbounded_channel();
        let bot = Arc::new(Self::new("1".to_string(), edit_tx));
        (bot, edit_rx)
    }
}

#[async_trait]
impl Bot for MockBot {
    async fn send_message(&self, _chat: &Chat, _text: &str) -> Result<()> {
        Ok(())
    }

    async fn reply_to(&self, _message: &Message, _text: &str) -> Result<()> {
        Ok(())
    }

    async fn edit_message(&self, chat: &Chat, message_id: &str, text: &str) -> Result<()> {
        let _ = self.edit_tx.send(EditRecord {
            chat_id: chat.id,
            message_id: message_id.to_string(),
            text: text.to_string(),
        });
        Ok(())
    }

    async fn send_message_and_return_id(&self, _chat: &Chat, _text: &str) -> Result<String> {
        Ok(self.placeholder_id.clone())
    }
}
