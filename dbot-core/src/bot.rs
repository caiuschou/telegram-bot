//! Bot abstraction for sending and editing messages.
//!
//! [`Bot`] trait is transport-agnostic; [`TelegramBot`] implements it via teloxide.

use crate::error::{DbotError, Result};
use crate::types::{Chat, Message};
use async_trait::async_trait;
use teloxide::{prelude::*, types::ChatId, types::MessageId};

/// Abstraction for sending and editing messages. Implementations map to a transport (e.g. Telegram).
#[async_trait]
pub trait Bot: Send + Sync {
    /// Sends a text message to the given chat.
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()>;
    /// Sends a reply to the given message (same chat).
    async fn reply_to(&self, message: &Message, text: &str) -> Result<()>;
    /// Edits an already-sent message (e.g. for streamed replies: send then edit). `message_id` is transport-specific (e.g. Telegram numeric string).
    async fn edit_message(&self, chat: &Chat, message_id: &str, text: &str) -> Result<()>;
    /// Sends a message and returns its id (for later `edit_message` when streaming). May return empty string if not supported.
    async fn send_message_and_return_id(&self, chat: &Chat, text: &str) -> Result<String>;
}

/// Teloxide-based implementation of [`Bot`].
pub struct TelegramBot {
    bot: teloxide::Bot,
}

impl TelegramBot {
    /// Creates a bot using the given Telegram bot token.
    pub fn new(token: String) -> Self {
        Self {
            bot: teloxide::Bot::new(token),
        }
    }
}

#[async_trait]
impl Bot for TelegramBot {
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()> {
        self.bot
            .send_message(ChatId(chat.id), text)
            .await
            .map_err(|e| DbotError::Bot(e.to_string()))?;
        Ok(())
    }

    async fn reply_to(&self, message: &Message, text: &str) -> Result<()> {
        self.send_message(&message.chat, text).await
    }

    async fn edit_message(&self, chat: &Chat, message_id: &str, text: &str) -> Result<()> {
        let id: i32 = message_id.parse().map_err(|_| {
            DbotError::Bot(format!("Invalid message_id for edit: {}", message_id))
        })?;
        self.bot
            .edit_message_text(ChatId(chat.id), MessageId(id), text)
            .await
            .map_err(|e| DbotError::Bot(e.to_string()))?;
        Ok(())
    }

    async fn send_message_and_return_id(&self, chat: &Chat, text: &str) -> Result<String> {
        let sent = self
            .bot
            .send_message(ChatId(chat.id), text)
            .await
            .map_err(|e| DbotError::Bot(e.to_string()))?;
        Ok(sent.id.to_string())
    }
}
