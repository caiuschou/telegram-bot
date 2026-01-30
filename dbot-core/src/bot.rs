use crate::error::{DbotError, Result};
use crate::types::{Chat, Message};
use async_trait::async_trait;
use teloxide::{prelude::*, types::ChatId, types::MessageId};

#[async_trait]
pub trait Bot: Send + Sync {
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()>;
    async fn reply_to(&self, message: &Message, text: &str) -> Result<()>;
    /// 编辑已发送的消息（用于流式回复「先发一条再编辑」）。message_id 与传输层一致（如 Telegram 为数字字符串）。
    async fn edit_message(&self, chat: &Chat, message_id: &str, text: &str) -> Result<()>;
    /// 发送消息并返回该消息的 id（用于流式时后续 edit_message）。无此能力时可返回空字符串或占位。
    async fn send_message_and_return_id(&self, chat: &Chat, text: &str) -> Result<String>;
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
