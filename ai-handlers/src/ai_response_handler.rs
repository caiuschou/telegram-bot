use dbot_core::Result;
use embedding::EmbeddingService;
use memory::{
    ContextBuilder, MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, RecentMessagesStrategy,
    SemanticSearchStrategy, UserPreferencesStrategy,
};
use std::sync::Arc;
use storage::MessageRepository;
use telegram_bot_ai::TelegramBotAI;
use teloxide::{prelude::*, Bot};
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{debug, error, info};
use chrono::Utc;

/// Handler for AI queries: receives `AIQuery` messages, builds context, calls the AI, sends replies, and persists to memory.
///
/// **External interactions:**
/// - **Telegram Bot API** (via `bot`): send and edit messages in the chat.
/// - **MessageRepository** (via `repo`): read messages (reply target, recent by chat) and write AI response logs.
/// - **MemoryStore** (via `memory_store`): add user/assistant turns and read history for context building.
/// - **EmbeddingService** (via `embedding_service`): embed the user question for semantic search over the vector store.
/// - **TelegramBotAI** (via `ai_bot`): call the LLM for one-shot or streaming responses.
pub struct AIQueryHandler {
    ai_bot: TelegramBotAI,
    bot: Arc<Bot>,
    repo: MessageRepository,
    memory_store: Arc<dyn MemoryStore>,
    /// Used by `build_memory_context` to embed the user question and run semantic search over the vector store.
    embedding_service: Arc<dyn EmbeddingService>,
    receiver: UnboundedReceiver<crate::ai_mention_detector::AIQuery>,
    use_streaming: bool,
    thinking_message: String,
}

impl AIQueryHandler {
    /// Constructs an `AIQueryHandler`. Does not perform network or I/O; only stores dependencies.
    ///
    /// **Parameters:**
    /// - `receiver`: channel receiver for `AIQuery` messages produced by `AIDetectionHandler`.
    /// - `use_streaming`: when `true`, replies are streamed (one message is sent and then edited as chunks arrive).
    /// - `thinking_message`: placeholder text sent at the start of streaming (e.g. "Thinking...").
    pub fn new(
        ai_bot: TelegramBotAI,
        bot: Bot,
        repo: MessageRepository,
        memory_store: Arc<dyn MemoryStore>,
        embedding_service: Arc<dyn EmbeddingService>,
        receiver: UnboundedReceiver<crate::ai_mention_detector::AIQuery>,
        use_streaming: bool,
        thinking_message: String,
    ) -> Self {
        Self {
            ai_bot,
            bot: Arc::new(bot),
            repo,
            memory_store,
            embedding_service,
            receiver,
            use_streaming,
            thinking_message,
        }
    }

    /// Main loop: continuously receives `AIQuery` from `receiver` and processes each with `handle_query`.
    /// Only interacts with `AIDetectionHandler` via the `UnboundedReceiver`; no other external calls in this method.
    pub async fn run(&mut self) {
        while let Some(query) = self.receiver.recv().await {
            self.handle_query(query).await;
        }
    }

    /// Processes a single AI query: saves the user message to memory, then dispatches to `handle_query_normal` or `handle_query_streaming` based on runtime `use_streaming`.
    async fn handle_query(&self, query: crate::ai_mention_detector::AIQuery) {
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

    /// Non-streaming path: builds memory context, formats question with context, calls the AI once, sends the reply, and saves the assistant turn to memory.
    /// **External calls:** `build_memory_context` (MemoryStore, EmbeddingService), `TelegramBotAI::get_ai_response`, `send_response` (Bot, MessageRepository), `save_to_memory` (MemoryStore).
    async fn handle_query_normal(&self, query: crate::ai_mention_detector::AIQuery) {
        let user_id_str = query.user_id.to_string();
        let conversation_id_str = query.chat_id.to_string();

        debug!(
            user_id = %user_id_str,
            conversation_id = %conversation_id_str,
            question = %query.question,
            "Processing AI query"
        );

        let context = self
            .build_memory_context(&user_id_str, &conversation_id_str, &query.question)
            .await;
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

    /// Streaming path: sends a "thinking" placeholder message, then streams AI chunks into the same message via edit, and finally saves the full assistant reply to memory.
    /// **External calls:** `bot.send_message` (placeholder), `bot.edit_message_text` (per chunk), `TelegramBotAI::get_ai_response_stream`, `save_to_memory` (MemoryStore).
    async fn handle_query_streaming(&self, query: crate::ai_mention_detector::AIQuery) {
        let user_id_str = query.user_id.to_string();
        let conversation_id_str = query.chat_id.to_string();

        info!(user_id = %user_id_str, question = %query.question, "Processing AI query (streaming mode)");

        let context = self
            .build_memory_context(&user_id_str, &conversation_id_str, &query.question)
            .await;
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

    /// Sends the AI reply to the chat identified by `query.chat_id` and logs it in the repository as an `ai_response` record.
    /// **External calls:** `bot.send_message`, then `log_ai_response` (MessageRepository::save).
    async fn send_response(
        &self,
        query: &crate::ai_mention_detector::AIQuery,
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

    /// Persists the AI reply as a single MessageRepository record with `kind = "ai_response"` and `status = "sent"` for audit and tracing. Does not send to Telegram; only writes to the repository.
    async fn log_ai_response(
        &self,
        query: &crate::ai_mention_detector::AIQuery,
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

    /// Builds a context string from "replied-to message" and "recent messages in the chat" for display or legacy behaviour. If `query.reply_to_message_id` is set, fetches that message and formats it as a "[回复消息]" block; then fetches up to 10 recent messages for the chat and appends a "[最近的消息]" block. **External calls:** MessageRepository::get_message_by_id, MessageRepository::get_recent_messages_by_chat.
    pub async fn build_context(&self, query: &crate::ai_mention_detector::AIQuery) -> String {
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

    /// Formats the prompt sent to the model: if `context` is empty returns `question` as-is; otherwise returns `context` followed by `"\n\n用户提问: "` and `question`. No external calls; pure string concatenation.
    pub fn format_question_with_context(&self, question: &str, context: &str) -> String {
        if context.is_empty() {
            question.to_string()
        } else {
            format!("{}\n\n用户提问: {}", context, question)
        }
    }

    /// Builds context from memory: recent messages, user preferences, and semantic search over the vector store using `question`, then merged and truncated to the token limit. Uses ContextBuilder with RecentMessagesStrategy, SemanticSearchStrategy (which calls EmbeddingService to embed `question`), and UserPreferencesStrategy. **External calls:** MemoryStore (recent + semantic search), EmbeddingService::embed / EmbeddingService::embed_batch.
    pub(crate) async fn build_memory_context(
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

    /// Writes one turn (user or assistant) to the MemoryStore. Metadata includes `user_id`, `conversation_id`, `role`, and `timestamp` from the current query. **Only interacts with MemoryStore** (MemoryStore::add). On failure logs the error and does not propagate it.
    pub(crate) async fn save_to_memory(&self, query: &crate::ai_mention_detector::AIQuery, content: &str, role: MemoryRole) {
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
