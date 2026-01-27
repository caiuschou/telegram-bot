use dbot_core::Result;
use memory::{ContextBuilder, MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, RecentMessagesStrategy, UserPreferencesStrategy};
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::{prelude::*, Bot};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info, debug};
use chrono::Utc;

pub struct AIQueryHandler {
    ai_bot: TelegramBotAI,
    bot: Arc<Bot>,
    repo: MessageRepository,
    memory_store: Arc<dyn MemoryStore>,
    receiver: UnboundedReceiver<crate::ai_detection_handler::AIQuery>,
    use_streaming: bool,
    thinking_message: String,
}

impl AIQueryHandler {
    pub fn new(
        ai_bot: TelegramBotAI,
        bot: Bot,
        repo: MessageRepository,
        memory_store: Arc<dyn MemoryStore>,
        receiver: UnboundedReceiver<crate::ai_detection_handler::AIQuery>,
        use_streaming: bool,
        thinking_message: String,
    ) -> Self {
        Self {
            ai_bot,
            bot: Arc::new(bot),
            repo,
            memory_store,
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

        self.save_to_memory(&query, &query.question, MemoryRole::User).await;

        if self.use_streaming {
            self.handle_query_streaming(query).await
        } else {
            self.handle_query_normal(query).await
        }
    }

    async fn handle_query_normal(&self, query: crate::ai_detection_handler::AIQuery) {
        let user_id_str = query.user_id.to_string();
        let conversation_id_str = query.chat_id.to_string();

        debug!(
            user_id = %user_id_str,
            conversation_id = %conversation_id_str,
            question = %query.question,
            "Processing AI query"
        );

        let context = self.build_memory_context(&user_id_str, &conversation_id_str).await;
        let question_with_context = self.format_question_with_context(&query.question, &context);

        match self.ai_bot.get_ai_response(&question_with_context).await {
            Ok(response) => {
                if let Err(e) = self.send_response(&query, &response).await {
                    error!(error = %e, "Failed to send AI response");
                } else {
                    self.save_to_memory(&query, &response, MemoryRole::Assistant).await;
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
        let user_id_str = query.user_id.to_string();
        let conversation_id_str = query.chat_id.to_string();

        info!(user_id = %user_id_str, question = %query.question, "Processing AI query (streaming mode)");

        let context = self.build_memory_context(&user_id_str, &conversation_id_str).await;
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
                        self.save_to_memory(&query, &full_response, MemoryRole::Assistant).await;
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

    async fn build_memory_context(&self, user_id: &str, conversation_id: &str) -> String {
        let builder = ContextBuilder::new(self.memory_store.clone())
            .with_strategy(Box::new(RecentMessagesStrategy::new(10)))
            .with_strategy(Box::new(UserPreferencesStrategy::new()))
            .with_token_limit(4096)
            .for_user(user_id)
            .for_conversation(conversation_id);

        match builder.build().await {
            Ok(context) => {
                if context.conversation_history.is_empty() {
                    String::new()
                } else {
                    context.format_for_model(false)
                }
            }
            Err(e) => {
                error!(error = %e, "Failed to build memory context");
                String::new()
            }
        }
    }

    async fn save_to_memory(&self, query: &crate::ai_detection_handler::AIQuery, content: &str, role: MemoryRole) {
        let metadata = MemoryMetadata {
            user_id: Some(query.user_id.to_string()),
            conversation_id: Some(query.chat_id.to_string()),
            role,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };

        let entry = MemoryEntry::new(content.to_string(), metadata);

        if let Err(e) = self.memory_store.add(entry).await {
            error!(error = %e, "Failed to save to memory");
        } else {
            debug!(
                user_id = query.user_id,
                conversation_id = query.chat_id,
                role = ?role,
                "Saved to memory"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory::InMemoryVectorStore;
    use openai_client::OpenAIClient;
    use storage::MessageRecord;
    use std::sync::Arc;

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

        let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo.clone(),
            memory_store,
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
        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store,
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
        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store,
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
        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store,
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
        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store,
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

    #[tokio::test]
    async fn test_save_to_memory_user_query() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

        let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store.clone(),
            _rx,
            false,
            "Thinking...".to_string(),
        );

        let query = crate::ai_detection_handler::AIQuery {
            chat_id: 123,
            user_id: 456,
            question: "What is AI?".to_string(),
            reply_to_message_id: None,
        };

        handler.save_to_memory(&query, "What is AI?", MemoryRole::User).await;

        let entries = memory_store.search_by_user("456").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "What is AI?");
        assert_eq!(entries[0].metadata.role, MemoryRole::User);
    }

    #[tokio::test]
    async fn test_save_to_memory_ai_response() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

        let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store.clone(),
            _rx,
            false,
            "Thinking...".to_string(),
        );

        let query = crate::ai_detection_handler::AIQuery {
            chat_id: 123,
            user_id: 456,
            question: "What is AI?".to_string(),
            reply_to_message_id: None,
        };

        handler.save_to_memory(&query, "AI is artificial intelligence.", MemoryRole::Assistant).await;

        let entries = memory_store.search_by_user("456").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "AI is artificial intelligence.");
        assert_eq!(entries[0].metadata.role, MemoryRole::Assistant);
    }

    #[tokio::test]
    async fn test_build_memory_context_empty() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

        let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store,
            _rx,
            false,
            "Thinking...".to_string(),
        );

        let context = handler.build_memory_context("123", "456").await;
        assert!(context.is_empty());
    }

    #[tokio::test]
    async fn test_build_memory_context_with_history() {
        let database_url = "sqlite::memory:";
        let repo = MessageRepository::new(database_url)
            .await
            .expect("Failed to create repository");

        let memory_store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn memory::MemoryStore>;

        let query = crate::ai_detection_handler::AIQuery {
            chat_id: 456,
            user_id: 123,
            question: "Hello".to_string(),
            reply_to_message_id: None,
        };

        let (_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let openai_client = OpenAIClient::new("test_key".to_string());
        let ai_bot = telegram_bot_ai::TelegramBotAI::new("test_bot".to_string(), openai_client);
        let handler = AIQueryHandler::new(
            ai_bot,
            teloxide::Bot::new("test_token"),
            repo,
            memory_store.clone(),
            _rx,
            false,
            "Thinking...".to_string(),
        );

        handler.save_to_memory(&query, "What is AI?", MemoryRole::User).await;

        let context = handler.build_memory_context("123", "456").await;
        assert!(!context.is_empty());
        assert!(context.contains("What is AI?"));
    }
}
