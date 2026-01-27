use dbot_core::Result;
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::{prelude::*, Bot};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info};

pub struct AIQueryHandler {
    ai_bot: TelegramBotAI,
    bot: Arc<Bot>,
    repo: MessageRepository,
    receiver: UnboundedReceiver<crate::ai_detection_handler::AIQuery>,
    use_streaming: bool,
    thinking_message: String,
}

impl AIQueryHandler {
    pub fn new(
        ai_bot: TelegramBotAI,
        bot: Bot,
        repo: MessageRepository,
        receiver: UnboundedReceiver<crate::ai_detection_handler::AIQuery>,
        use_streaming: bool,
        thinking_message: String,
    ) -> Self {
        Self {
            ai_bot,
            bot: Arc::new(bot),
            repo,
            receiver,
            use_streaming,
            thinking_message,
        }
    }

    pub async fn run(&mut self) {
        while let Some(query) = self.receiver.recv().await {
            self.handle_query(query).await;
        }
    }

    async fn handle_query(&self, query: crate::ai_detection_handler::AIQuery) {
        info!(
            user_id = query.user_id,
            question = %query.question,
            use_streaming = self.use_streaming,
            "Processing AI query"
        );

        if self.use_streaming {
            self.handle_query_streaming(query).await
        } else {
            self.handle_query_normal(query).await
        }
    }

    async fn handle_query_normal(&self, query: crate::ai_detection_handler::AIQuery) {
        match self.ai_bot.get_ai_response(&query.question).await {
            Ok(response) => {
                if let Err(e) = self.send_response(&query, &response).await {
                    error!(error = %e, "Failed to send AI response");
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to get AI response");
                let error_msg = "抱歉，处理您的请求时出错，请稍后重试。";
                let _ = self.send_response(&query, error_msg).await;
            }
        }
    }

    async fn handle_query_streaming(&self, query: crate::ai_detection_handler::AIQuery) {
        info!(user_id = query.user_id, question = %query.question, "Processing AI query (streaming mode)");

        let chat_id = ChatId(query.chat_id);

        match self.bot.send_message(chat_id, &self.thinking_message).await {
            Ok(msg) => {
                let message_id = msg.id;
                let bot = self.bot.clone();
                let full_content = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));

                match self.ai_bot.get_ai_response_stream(&query.question, |chunk| {
                    let bot = bot.clone();
                    let full_content = full_content.clone();
                    async move {
                        if !chunk.content.is_empty() {
                            let mut content = full_content.lock().await;
                            content.push_str(&chunk.content);
                            if let Err(e) = bot.edit_message_text(chat_id, message_id, &*content).await {
                                error!(error = %e, "Failed to edit message");
                                return Err(anyhow::anyhow!("Failed to edit message: {}", e));
                            }
                        }
                        Ok(())
                    }
                }).await {
                    Ok(full_response) => {
                        let _ = self.log_ai_response(&query, &full_response).await;
                    }
                    Err(e) => {
                        error!(error = %e, "AI stream response failed");
                        let _ = self.bot.edit_message_text(chat_id, message_id, "抱歉，AI 响应失败。").await;
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to send thinking message");
                let error_msg = "抱歉，处理您的请求时出错，请稍后重试。";
                let _ = self.send_response(&query, error_msg).await;
            }
        }
    }

    async fn send_response(
        &self,
        query: &crate::ai_detection_handler::AIQuery,
        response: &str,
    ) -> Result<()> {
        let chat_id = ChatId(query.chat_id);

        self.bot
            .send_message(chat_id, response)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to send message");
                dbot_core::DbotError::Bot(e.to_string())
            })?;

        self.log_ai_response(query, response).await?;

        info!(user_id = query.user_id, "AI response sent");
        Ok(())
    }

    async fn log_ai_response(
        &self,
        query: &crate::ai_detection_handler::AIQuery,
        response: &str,
    ) -> Result<()> {
        let record = storage::MessageRecord::new(
            query.user_id,
            query.chat_id,
            None,
            None,
            None,
            "ai_response".to_string(),
            response.to_string(),
            "sent".to_string(),
        );

        self.repo
            .save(&record)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to save AI response");
                dbot_core::DbotError::Database(e.to_string())
            })?;

        Ok(())
    }
}
