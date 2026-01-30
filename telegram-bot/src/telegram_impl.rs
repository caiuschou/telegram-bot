//! TelegramBot: thin wrapper around teloxide::Bot implementing dbot_core::Bot. Used in production; tests can substitute another Bot impl.

use async_trait::async_trait;
use dbot_core::{Bot as CoreBot, Chat, Message, Result};
use teloxide::{prelude::*, types::ChatId, types::MessageId};

/// Thin wrapper around teloxide::Bot implementing dbot_core::Bot. Production code uses this to send messages; tests can replace with a mock.
pub struct TelegramBot {
    bot: teloxide::Bot,
}

impl TelegramBot {
    /// Creates a bot with the given Telegram bot token.
    pub fn new(token: String) -> Self {
        Self {
            bot: teloxide::Bot::new(token),
        }
    }

    /// Returns the underlying teloxide::Bot for direct API use.
    pub fn inner(&self) -> &teloxide::Bot {
        &self.bot
    }
}

#[async_trait]
impl CoreBot for TelegramBot {
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()> {
        self.bot
            .send_message(ChatId(chat.id), text.to_string())
            .await
            .map_err(|e| dbot_core::DbotError::Bot(e.to_string()))?;
        Ok(())
    }

    async fn send_message_and_return_id(&self, chat: &Chat, text: &str) -> Result<String> {
        let sent = self
            .bot
            .send_message(ChatId(chat.id), text.to_string())
            .await
            .map_err(|e| dbot_core::DbotError::Bot(e.to_string()))?;
        Ok(sent.id.to_string())
    }

    async fn reply_to(&self, message: &Message, text: &str) -> Result<()> {
        self.send_message(&message.chat, text).await
    }

    async fn edit_message(&self, chat: &Chat, message_id: &str, text: &str) -> Result<()> {
        let id: i32 = message_id.parse().map_err(|_| {
            dbot_core::DbotError::Bot(format!("Invalid message_id for edit: {}", message_id))
        })?;
        self.bot
            .edit_message_text(ChatId(chat.id), MessageId(id), text)
            .await
            .map_err(|e| dbot_core::DbotError::Bot(e.to_string()))?;
        Ok(())
    }
}
