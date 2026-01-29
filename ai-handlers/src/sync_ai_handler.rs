//! Synchronous AI handler: runs in the handler chain, calls AI and returns `HandlerResponse::Reply(text)` so middleware (e.g. MemoryMiddleware) can save the reply in `after()`.

use async_trait::async_trait;
use dbot_core::{Handler, HandlerResponse, Message, Result};
use embedding::EmbeddingService;
use memory::{
    ContextBuilder, MemoryStore, RecentMessagesStrategy, SemanticSearchStrategy,
    UserPreferencesStrategy,
};
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::{prelude::*, Bot};
use tracing::{debug, error, info, instrument};

/// Synchronous AI handler: when the message is an AI query (reply-to or @mention), builds context, calls the AI, sends the reply to Telegram, and returns `HandlerResponse::Reply(response_text)` so middleware can persist it (e.g. MemoryMiddleware in `after()`).
///
/// **External interactions:** Telegram Bot API (send/edit), MessageRepository (log), MemoryStore (context build), EmbeddingService (semantic search), TelegramBotAI (LLM).
#[derive(Clone)]
pub struct SyncAIHandler {
    pub(crate) bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    pub(crate) ai_bot: TelegramBotAI,
    pub(crate) bot: Arc<Bot>,
    pub(crate) repo: MessageRepository,
    pub(crate) memory_store: Arc<dyn MemoryStore>,
    pub(crate) embedding_service: Arc<dyn EmbeddingService>,
    pub(crate) use_streaming: bool,
    pub(crate) thinking_message: String,
}

impl SyncAIHandler {
    pub fn new(
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        ai_bot: TelegramBotAI,
        bot: Bot,
        repo: MessageRepository,
        memory_store: Arc<dyn MemoryStore>,
        embedding_service: Arc<dyn EmbeddingService>,
        use_streaming: bool,
        thinking_message: String,
    ) -> Self {
        Self {
            bot_username,
            ai_bot,
            bot: Arc::new(bot),
            repo,
            memory_store,
            embedding_service,
            use_streaming,
            thinking_message,
        }
    }

    async fn get_bot_username(&self) -> Option<String> {
        self.bot_username.read().await.clone()
    }

    fn is_bot_mentioned(&self, text: &str, bot_username: &str) -> bool {
        text.contains(&format!("@{}", bot_username))
    }

    fn extract_question(&self, text: &str, bot_username: &str) -> String {
        text.replace(&format!("@{}", bot_username), "")
            .trim()
            .to_string()
    }

    fn get_question(&self, message: &Message, bot_username: Option<&str>) -> Option<String> {
        if message.reply_to_message_id.is_some() {
            return Some(message.content.clone());
        }
        if let Some(username) = bot_username {
            if self.is_bot_mentioned(&message.content, username) {
                let q = self.extract_question(&message.content, username);
                if !q.is_empty() {
                    return Some(q);
                }
            }
        }
        None
    }

    async fn build_memory_context(
        &self,
        user_id: &str,
        conversation_id: &str,
        question: &str,
    ) -> String {
        let builder = ContextBuilder::new(self.memory_store.clone())
            .with_strategy(Box::new(RecentMessagesStrategy::new(10)))
            .with_strategy(Box::new(SemanticSearchStrategy::new(
                5,
                self.embedding_service.clone(),
            )))
            .with_strategy(Box::new(UserPreferencesStrategy::new()))
            .with_token_limit(4096)
            .for_user(user_id)
            .for_conversation(conversation_id)
            .with_query(question);

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

    fn format_question_with_context(&self, question: &str, context: &str) -> String {
        if context.is_empty() {
            question.to_string()
        } else {
            format!("{}\n\n用户提问: {}", context, question)
        }
    }

    async fn send_response_for_message(&self, message: &Message, response: &str) -> Result<()> {
        let chat_id = ChatId(message.chat.id);
        self.bot
            .send_message(chat_id, response)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to send message");
                dbot_core::DbotError::Bot(e.to_string())
            })?;
        self.log_ai_response_for_message(message, response).await?;
        info!(user_id = message.user.id, "AI response sent");
        Ok(())
    }

    async fn log_ai_response_for_message(&self, message: &Message, response: &str) -> Result<()> {
        let record = storage::MessageRecord::new(
            message.user.id,
            message.chat.id,
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

    async fn process_normal(&self, message: &Message, question: &str) -> Result<HandlerResponse> {
        let user_id_str = message.user.id.to_string();
        let conversation_id_str = message.chat.id.to_string();

        let context = self
            .build_memory_context(&user_id_str, &conversation_id_str, question)
            .await;
        let question_with_context = self.format_question_with_context(question, &context);

        match self.ai_bot.get_ai_response(&question_with_context).await {
            Ok(response) => {
                if let Err(e) = self.send_response_for_message(message, &response).await {
                    error!(error = %e, "Failed to send AI response");
                    let _ = self
                        .send_response_for_message(message, "抱歉，发送回复时出错。")
                        .await;
                    return Ok(HandlerResponse::Stop);
                }
                Ok(HandlerResponse::Reply(response))
            }
            Err(e) => {
                error!(error = %e, "Failed to get AI response");
                let _ = self
                    .send_response_for_message(message, "抱歉，处理您的请求时出错，请稍后重试。")
                    .await;
                Ok(HandlerResponse::Stop)
            }
        }
    }

    async fn process_streaming(&self, message: &Message, question: &str) -> Result<HandlerResponse> {
        let user_id_str = message.user.id.to_string();
        let conversation_id_str = message.chat.id.to_string();

        debug!(
            user_id = %user_id_str,
            conversation_id = %conversation_id_str,
            question = %question,
            "Processing AI query (streaming)"
        );

        let context = self
            .build_memory_context(&user_id_str, &conversation_id_str, question)
            .await;
        let question_with_context = self.format_question_with_context(question, &context);

        let chat_id = ChatId(message.chat.id);

        let msg = match self.bot.send_message(chat_id, &self.thinking_message).await {
            Ok(m) => m,
            Err(e) => {
                error!(error = %e, "Failed to send thinking message");
                let _ = self
                    .send_response_for_message(message, "抱歉，处理您的请求时出错，请稍后重试。")
                    .await;
                return Ok(HandlerResponse::Stop);
            }
        };

        let message_id = msg.id;
        let bot = self.bot.clone();
        let full_content = Arc::new(tokio::sync::Mutex::new(String::new()));

        match self
            .ai_bot
            .get_ai_response_stream(&question_with_context, |chunk| {
                let bot = bot.clone();
                let full_content = full_content.clone();
                async move {
                    if !chunk.content.is_empty() {
                        let mut content = full_content.lock().await;
                        content.push_str(&chunk.content);
                        if let Err(e) =
                            bot.edit_message_text(chat_id, message_id, &*content).await
                        {
                            error!(error = %e, "Failed to edit message");
                            return Err(anyhow::anyhow!("Failed to edit message: {}", e));
                        }
                    }
                    Ok(())
                }
            })
            .await
        {
            Ok(full_response) => {
                let _ = self.log_ai_response_for_message(message, &full_response).await;
                Ok(HandlerResponse::Reply(full_response))
            }
            Err(e) => {
                error!(error = %e, "AI stream response failed");
                let _ = self
                    .bot
                    .edit_message_text(chat_id, message_id, "抱歉，AI 响应失败。")
                    .await;
                Ok(HandlerResponse::Stop)
            }
        }
    }
}

#[async_trait]
impl Handler for SyncAIHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let bot_username = self.get_bot_username().await;
        let question = match self.get_question(message, bot_username.as_deref()) {
            Some(q) => q,
            None => return Ok(HandlerResponse::Continue),
        };

        if message.reply_to_message_id.is_some() {
            info!(
                user_id = message.user.id,
                reply_to = ?message.reply_to_message_id,
                "Replying to bot message, processing synchronously"
            );
        } else {
            info!(
                user_id = message.user.id,
                question = %question,
                "Bot mentioned, processing synchronously"
            );
        }

        if self.use_streaming {
            self.process_streaming(message, &question).await
        } else {
            self.process_normal(message, &question).await
        }
    }
}
