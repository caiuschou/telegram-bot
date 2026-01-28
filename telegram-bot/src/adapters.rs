use dbot_core::{Chat, Message, MessageDirection, ToCoreMessage, ToCoreUser, User};

/// Telegram 用户到 Core 用户的转换器
pub struct TelegramUserWrapper<'a>(pub &'a teloxide::types::User);

impl<'a> ToCoreUser for TelegramUserWrapper<'a> {
    fn to_core(&self) -> User {
        User {
            id: self.0.id.0 as i64,
            username: self.0.username.clone(),
            first_name: Some(self.0.first_name.clone()),
            last_name: self.0.last_name.clone(),
        }
    }
}

/// Telegram 消息到 Core 消息的转换器
pub struct TelegramMessageWrapper<'a>(pub &'a teloxide::types::Message);

impl<'a> ToCoreMessage for TelegramMessageWrapper<'a> {
    fn to_core(&self) -> Message {
        Message {
            id: self.0.id.to_string(),
            user: self
                .0
                .from
                .as_ref()
                .map(|u| TelegramUserWrapper(u).to_core())
                .unwrap_or_else(|| User {
                    id: 0,
                    username: None,
                    first_name: None,
                    last_name: None,
                }),
            chat: Chat {
                id: self.0.chat.id.0,
                chat_type: format!("{:?}", self.0.chat.kind),
            },
            content: self.0.text().unwrap_or("").to_string(),
            message_type: "text".to_string(),
            direction: MessageDirection::Incoming,
            created_at: chrono::Utc::now(),
            reply_to_message_id: self.get_reply_to_message_id(),
        }
    }
}

impl<'a> TelegramMessageWrapper<'a> {
    /// 获取被回复消息的 ID
    fn get_reply_to_message_id(&self) -> Option<String> {
        self.0.reply_to_message().map(|msg| msg.id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_user_wrapper_to_core() {
        let user = teloxide::types::User {
            id: teloxide::types::UserId(123),
            is_bot: false,
            first_name: "Test".to_string(),
            last_name: Some("User".to_string()),
            username: Some("testuser".to_string()),
            language_code: Some("en".to_string()),
            is_premium: false,
            added_to_attachment_menu: false,
        };

        let wrapper = TelegramUserWrapper(&user);
        let core_user = wrapper.to_core();

        assert_eq!(core_user.id, 123);
        assert_eq!(core_user.username, Some("testuser".to_string()));
        assert_eq!(core_user.first_name, Some("Test".to_string()));
        assert_eq!(core_user.last_name, Some("User".to_string()));
    }

    #[test]
    fn test_telegram_user_wrapper_minimal() {
        let user = teloxide::types::User {
            id: teloxide::types::UserId(456),
            is_bot: false,
            first_name: "Minimal".to_string(),
            last_name: None,
            username: None,
            language_code: None,
            is_premium: false,
            added_to_attachment_menu: false,
        };

        let wrapper = TelegramUserWrapper(&user);
        let core_user = wrapper.to_core();

        assert_eq!(core_user.id, 456);
        assert_eq!(core_user.username, None);
        assert_eq!(core_user.first_name, Some("Minimal".to_string()));
        assert_eq!(core_user.last_name, None);
    }
}
