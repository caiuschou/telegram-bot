//! 图片生成 Handler：检测图片生成请求并调用 DALL-E API 生成图片

use async_trait::async_trait;
use dbot_core::{Bot as CoreBot, Handler, HandlerResponse, Message, Result};
use image_generation_client::ImageGenerationClient;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

// --- 用户友好的错误消息 ---
const MSG_SEND_FAILED: &str = "抱歉，发送图片时出错。";
const MSG_GENERATION_FAILED: &str = "抱歉，图片生成失败，请稍后重试。";
const MSG_INVALID_PROMPT: &str = "请输入有效的图片描述。";

/// 图片生成触发关键词
const IMAGE_GENERATION_TRIGGERS: &[&str] = &["画", "生成图片", "生成图像", "画图", "/image", "/draw"];

/// 图片生成 Handler
/// 
/// 检测用户消息中的图片生成请求（如包含"画"、"生成图片"等关键词），
/// 调用 DALL-E API 生成图片并发送给用户。
#[derive(Clone)]
pub struct ImageGenerationHandler {
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    image_client: Arc<ImageGenerationClient>,
    bot: Arc<dyn CoreBot>,
}

impl ImageGenerationHandler {
    /// 创建新的图片生成 Handler
    pub fn new(
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        image_client: Arc<ImageGenerationClient>,
        bot: Arc<dyn CoreBot>,
    ) -> Self {
        Self {
            bot_username,
            image_client,
            bot,
        }
    }

    /// 检查消息是否包含图片生成请求
    fn is_image_generation_request(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        IMAGE_GENERATION_TRIGGERS.iter().any(|trigger| {
            text_lower.contains(trigger)
        })
    }

    /// 提取图片描述（移除触发词）
    async fn extract_prompt(&self, text: &str) -> Option<String> {
        let text_lower = text.to_lowercase();
        for trigger in IMAGE_GENERATION_TRIGGERS {
            if text_lower.contains(trigger) {
                // 移除触发词，保留其余内容作为 prompt
                let prompt = text
                    .replace(trigger, "")
                    .replace(&trigger.to_uppercase(), "")
                    .trim()
                    .to_string();
                if !prompt.is_empty() {
                    return Some(prompt);
                }
            }
        }
        // 如果没有找到触发词，但消息以 @bot 开头，尝试提取 @ 后的内容
        if let Some(bot_username) = self.bot_username.read().await.as_ref() {
            if text.contains(&format!("@{}", bot_username)) {
                let prompt = text
                    .replace(&format!("@{}", bot_username), "")
                    .trim()
                    .to_string();
                if !prompt.is_empty() {
                    return Some(prompt);
                }
            }
        }
        None
    }

    /// 处理图片生成请求
    async fn handle_image_generation(&self, message: &Message, prompt: &str) -> Result<HandlerResponse> {
        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            prompt_preview = %prompt.chars().take(50).collect::<String>(),
            "Processing image generation request"
        );

        // 发送"正在生成"提示
        let thinking_msg = "正在生成图片，请稍候...";
        if let Err(e) = self.bot.send_message(&message.chat, thinking_msg).await {
            error!(error = %e, "Failed to send thinking message");
            return self.send_fallback_and_stop(message, MSG_SEND_FAILED).await;
        }

        // 调用图片生成 API
        let image_url = match self.image_client.generate_image(prompt).await {
            Ok(url) => url,
            Err(e) => {
                error!(error = %e, "Image generation failed");
                return self.send_fallback_and_stop(message, MSG_GENERATION_FAILED).await;
            }
        };

        // 发送生成的图片
        let caption = format!("根据描述生成的图片：{}", prompt);
        if let Err(e) = self.bot.send_photo(&message.chat, &image_url, Some(&caption)).await {
            error!(error = %e, "Failed to send photo");
            return self.send_fallback_and_stop(message, MSG_SEND_FAILED).await;
        }

        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            image_url = %image_url,
            "Image generated and sent successfully"
        );

        // 返回 Stop 表示已处理，不需要继续传递
        Ok(HandlerResponse::Stop)
    }

    /// 发送错误消息并停止处理
    async fn send_fallback_and_stop(&self, message: &Message, text: &str) -> Result<HandlerResponse> {
        let _ = self.bot.send_message(&message.chat, text).await;
        Ok(HandlerResponse::Stop)
    }
}

#[async_trait]
impl Handler for ImageGenerationHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        // 检查是否为图片生成请求
        if !self.is_image_generation_request(&message.content) {
            return Ok(HandlerResponse::Continue);
        }

        // 提取图片描述
        let prompt = match self.extract_prompt(&message.content).await {
            Some(p) => p,
            None => {
                debug!("No valid prompt extracted, skipping");
                return Ok(HandlerResponse::Continue);
            }
        };

        // 处理图片生成
        self.handle_image_generation(message, &prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dbot_core::{Chat, Message, MessageDirection, User};
    use std::sync::Arc;

    struct MockBot;
    struct MockImageClient;

    #[async_trait]
    impl CoreBot for MockBot {
        async fn send_message(&self, _chat: &Chat, _text: &str) -> Result<()> {
            Ok(())
        }
        async fn reply_to(&self, _message: &Message, _text: &str) -> Result<()> {
            Ok(())
        }
        async fn edit_message(&self, _chat: &Chat, _message_id: &str, _text: &str) -> Result<()> {
            Ok(())
        }
        async fn send_message_and_return_id(&self, _chat: &Chat, _text: &str) -> Result<String> {
            Ok("123".to_string())
        }
        async fn send_photo(&self, _chat: &Chat, _image_url: &str, _caption: Option<&str>) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_is_image_generation_request() {
        let handler = ImageGenerationHandler::new(
            Arc::new(tokio::sync::RwLock::new(None)),
            Arc::new(ImageGenerationClient::new("test".to_string())),
            Arc::new(MockBot),
        );

        assert!(handler.is_image_generation_request("画一只猫"));
        assert!(handler.is_image_generation_request("生成图片：风景"));
        assert!(handler.is_image_generation_request("请画图"));
        assert!(!handler.is_image_generation_request("普通消息"));
    }

    #[tokio::test]
    async fn test_extract_prompt() {
        let handler = ImageGenerationHandler::new(
            Arc::new(tokio::sync::RwLock::new(Some("mybot".to_string()))),
            Arc::new(ImageGenerationClient::new("test".to_string())),
            Arc::new(MockBot),
        );

        assert_eq!(
            handler.extract_prompt("画一只猫"),
            Some("一只猫".to_string())
        );
        assert_eq!(
            handler.extract_prompt("生成图片：美丽的风景"),
            Some("：美丽的风景".to_string())
        );
    }
}
