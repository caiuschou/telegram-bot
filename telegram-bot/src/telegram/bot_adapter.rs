//! Wraps teloxide::Bot and implements [`crate::core::Bot`]. Production code sends messages via Telegram; tests can substitute another Bot impl.

use crate::core::{Bot as CoreBot, DbotError, Chat, Message, Result};
use async_trait::async_trait;
use teloxide::{prelude::*, types::ChatId, types::MessageId};

/// Thin wrapper around teloxide::Bot that implements core's Bot trait.
pub struct TelegramBotAdapter {
    bot: teloxide::Bot,
}

impl TelegramBotAdapter {
    /// Creates an adapter from an existing teloxide Bot.
    pub fn new(bot: teloxide::Bot) -> Self {
        Self { bot }
    }

    /// Returns the underlying teloxide::Bot for direct API use when needed.
    pub fn inner(&self) -> &teloxide::Bot {
        &self.bot
    }
}

#[async_trait]
impl CoreBot for TelegramBotAdapter {
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()> {
        self.bot
            .send_message(ChatId(chat.id), text.to_string())
            .await
            .map_err(|e| DbotError::Bot(e.to_string()))?;
        Ok(())
    }

    async fn send_message_and_return_id(&self, chat: &Chat, text: &str) -> Result<String> {
        let sent = self
            .bot
            .send_message(ChatId(chat.id), text.to_string())
            .await
            .map_err(|e| DbotError::Bot(e.to_string()))?;
        Ok(sent.id.to_string())
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
}
