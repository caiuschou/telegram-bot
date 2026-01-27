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
        let context = self.build_context(&query).await;
        let question_with_context = self.format_question_with_context(&query.question, &context);

        match self.ai_bot.get_ai_response(&question_with_context).await {
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

        let context = self.build_context(&query).await;
        let question_with_context = self.format_question_with_context(&query.question, &context);

        let chat_id = ChatId(query.chat_id);

        match self.bot.send_message(chat_id, &self.thinking_message).await {
            Ok(msg) => {
                let message_id = msg.id;
                let bot = self.bot.clone();
                let full_content = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));

                match self.ai_bot.get_ai_response_stream(&question_with_context, |chunk| {
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

    pub async fn build_context(&self, query: &crate::ai_detection_handler::AIQuery) -> String {
        let mut context_parts = Vec::new();

        if let Some(ref reply_to_id) = query.reply_to_message_id {
            if let Ok(Some(replied_message)) = self.repo.get_message_by_id(reply_to_id).await {
                context_parts.push(format!(
                    "[回复消息]\n用户: {}\n内容: {}",
                    replied_message.username.unwrap_or_else(|| "未知".to_string()),
                    replied_message.content
                ));
            }
        }

        if let Ok(recent_messages) = self.repo.get_recent_messages_by_chat(query.chat_id, 10).await {
            if !recent_messages.is_empty() {
                let mut recent_context = String::from("\n[最近的消息]\n");
                for msg in recent_messages.iter().rev() {
                    let username = msg.username.clone().unwrap_or_else(|| "未知".to_string());
                    recent_context.push_str(&format!("{}: {}\n", username, msg.content));
                }
                context_parts.push(recent_context);
            }
        }

        context_parts.join("\n")
    }

    pub fn format_question_with_context(&self, question: &str, context: &str) -> String {
        if context.is_empty() {
            question.to_string()
        } else {
            format!("{}\n\n用户提问: {}", context, question)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_client::OpenAIClient;
    use storage::MessageRecord;

    #[tokio::test]
    async fn test_build_context_with_reply_to() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let replied_message = MessageRecord::new(
            123,
            456,
            Some("original_user".to_string()),
            Some("Original".to_string()),
            None,
            "text".to_string(),
            "This is the original message".to_string(),
            "received".to_string(),
        );

        repo.save(&replied_message)
            .await
            .expect("Failed to save replied message");

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo.clone(),
            _rx,
            false,
            "Thinking...".to_string(),
        );

        let query = crate::ai_detection_handler::AIQuery {
            chat_id: 456,
            user_id: 123,
            question: "Can you explain more?".to_string(),
            reply_to_message_id: Some(replied_message.id.clone()),
        };

        let context = handler.build_context(&query).await;

        assert!(context.contains("[回复消息]"));
        assert!(context.contains("original_user"));
        assert!(context.contains("This is the original message"));
    }

    #[tokio::test]
    async fn test_build_context_with_recent_messages() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let chat_id = 789;

        for i in 0..5 {
            let message = MessageRecord::new(
                100 + i,
                chat_id,
                Some(format!("user{}", i)),
                Some(format!("User{}", i)),
                None,
                "text".to_string(),
                format!("Message {}", i),
                "received".to_string(),
            );
            repo.save(&message)
                .await
                .expect("Failed to save message");
        }

        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            rx,
            false,
            "Thinking...".to_string(),
        );

        let query = crate::ai_detection_handler::AIQuery {
            chat_id,
            user_id: 100,
            question: "What was discussed?".to_string(),
            reply_to_message_id: None,
        };

        let context = handler.build_context(&query).await;

        assert!(context.contains("[最近的消息]"));
        assert!(context.contains("user0"));
        assert!(context.contains("Message 0"));
        assert!(context.contains("Message 4"));
    }

    #[tokio::test]
    async fn test_build_context_empty() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            rx,
            false,
            "Thinking...".to_string(),
        );

        let query = crate::ai_detection_handler::AIQuery {
            chat_id: 999,
            user_id: 123,
            question: "Hello".to_string(),
            reply_to_message_id: None,
        };

        let context = handler.build_context(&query).await;

        assert!(context.is_empty());
    }

    #[tokio::test]
    async fn test_format_question_with_context_empty() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            rx,
            false,
            "Thinking...".to_string(),
        );

        let question = "What is AI?";
        let context = "";
        let result = handler.format_question_with_context(question, context);

        assert_eq!(result, question);
    }

    #[tokio::test]
    async fn test_format_question_with_context_with_data() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            rx,
            false,
            "Thinking...".to_string(),
        );

        let question = "What is AI?";
        let context = "[回复消息]\n用户: test\n内容: Hello";
        let result = handler.format_question_with_context(question, context);

        assert!(result.contains(context));
        assert!(result.contains("用户提问:"));
        assert!(result.contains(question));
    }
}
