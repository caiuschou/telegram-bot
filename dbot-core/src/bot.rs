use crate::error::{DbotError, Result};
use crate::types::{Chat, Message};
use async_trait::async_trait;
use teloxide::{prelude::*, types::ChatId};

#[async_trait]
pub trait Bot {
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()>;
    async fn reply_to(&self, message: &Message, text: &str) -> Result<()>;
}

pub struct TelegramBot {
    bot: teloxide::Bot,
}

impl TelegramBot {
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
}
