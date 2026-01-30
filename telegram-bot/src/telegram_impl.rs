use async_trait::async_trait;
use dbot_core::{Bot as CoreBot, Chat, Message, Result};
use teloxide::{prelude::*, types::ChatId, types::MessageId};

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

    async fn send_photo(&self, chat: &Chat, image_url: &str, caption: Option<&str>) -> Result<()> {
        let mut request = self.bot.send_photo(ChatId(chat.id), image_url);
        if let Some(caption) = caption {
            request = request.caption(caption);
        }
        request
            .await
            .map_err(|e| dbot_core::DbotError::Bot(e.to_string()))?;
        Ok(())
    }
}
