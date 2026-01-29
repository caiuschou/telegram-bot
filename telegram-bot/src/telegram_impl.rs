use async_trait::async_trait;
use dbot_core::{Bot as CoreBot, Chat, Message, Result};
use teloxide::{prelude::*, types::ChatId};

/// 对 teloxide::Bot 的简单封装，实现 dbot-core 的 Bot trait。
///
/// - 在生产代码中用于真正向 Telegram 发送消息。
/// - 在测试中可以通过实现 dbot-core::Bot trait 的其他类型进行替换。
pub struct TelegramBot {
    bot: teloxide::Bot,
}

impl TelegramBot {
    /// 使用 Telegram Bot Token 创建新的 TelegramBot 实例。
    pub fn new(token: String) -> Self {
        Self {
            bot: teloxide::Bot::new(token),
        }
    }

    /// 暴露底层 teloxide::Bot，便于在需要时直接使用 Teloxide API。
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

    async fn reply_to(&self, message: &Message, text: &str) -> Result<()> {
        self.send_message(&message.chat, text).await
    }
}
