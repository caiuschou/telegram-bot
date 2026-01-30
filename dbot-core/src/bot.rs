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

/// Parses a message id string into an i32. Used by edit_message.
pub fn parse_message_id(s: &str) -> Result<i32> {
    s.parse().map_err(|_| {
        DbotError::Bot(format!("Invalid message_id for edit: {}", s))
    })
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
        let id = parse_message_id(message_id)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_bot_new() {
        let _bot = TelegramBot::new("dummy_token".to_string());
    }

    #[test]
    fn test_parse_message_id_valid() {
        assert_eq!(parse_message_id("123").unwrap(), 123);
        assert_eq!(parse_message_id("0").unwrap(), 0);
    }

    #[test]
    fn test_parse_message_id_invalid() {
        assert!(parse_message_id("").is_err());
        assert!(parse_message_id("abc").is_err());
        assert!(parse_message_id("12.3").is_err());
    }
}
