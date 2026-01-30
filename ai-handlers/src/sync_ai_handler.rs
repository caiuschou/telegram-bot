//! Synchronous AI handler: runs in the handler chain, calls AI and returns `HandlerResponse::Reply(text)` so middleware (e.g. MemoryMiddleware) can save the reply in `after()`.

use ai_client::{LlmClient, OpenAILlmClient};
use async_trait::async_trait;
use dbot_core::{Bot as CoreBot, Handler, HandlerResponse, Message, Result};
use embedding::EmbeddingService;
use memory::{
    Context, ContextBuilder, MemoryStore, RecentMessagesStrategy, SemanticSearchStrategy,
    UserPreferencesStrategy,
};
use prompt::ChatMessage;
use std::sync::Arc;
use storage::MessageRepository;
use tracing::{debug, error, info, instrument};

// --- User-facing fallback messages (sent to Telegram on errors) ---
const MSG_SEND_FAILED: &str = "抱歉，发送回复时出错。";
const MSG_REQUEST_FAILED: &str = "抱歉，处理您的请求时出错，请稍后重试。";
const MSG_STREAM_FAILED: &str = "抱歉，AI 响应失败。";

/// Logs the exact messages submitted to the AI (role + full content) for debugging.
fn log_messages_submitted_to_ai(messages: &[ChatMessage]) {
    info!(count = messages.len(), "submit_to_ai: 提交给 AI 的消息列表");
    for (i, m) in messages.iter().enumerate() {
        info!(
            index = i,
            role = ?m.role,
            content = %m.content,
            "submit_to_ai message"
        );
    }
}

/// Synchronous AI handler: when the message is an AI query (user replies to the bot's message, or @mentions the bot), builds context, calls the AI, sends the reply to Telegram, and returns `HandlerResponse::Reply(response_text)` so middleware can persist it (e.g. MemoryMiddleware in `after()`).
///
/// **External interactions:** Bot trait (send/edit), MessageRepository (log), MemoryStore (context build), EmbeddingService (semantic search), LlmClient (LLM).
#[derive(Clone)]
pub struct SyncAIHandler {
    pub(crate) bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    pub(crate) llm_client: Arc<OpenAILlmClient>,
    pub(crate) bot: Arc<dyn CoreBot>,
    pub(crate) repo: MessageRepository,
    pub(crate) memory_store: Arc<dyn MemoryStore>,
    /// When set, RecentMessagesStrategy and UserPreferencesStrategy use this store (e.g. SQLite); semantic search still uses `memory_store`.
    pub(crate) recent_store: Option<Arc<dyn MemoryStore>>,
    pub(crate) embedding_service: Arc<dyn EmbeddingService>,
    pub(crate) use_streaming: bool,
    pub(crate) thinking_message: String,
    /// 近期消息条数上限，用于 ContextBuilder 的 RecentMessagesStrategy（对应配置 MEMORY_RECENT_LIMIT）。
    pub(crate) memory_recent_limit: usize,
    /// 语义检索 Top-K，用于 ContextBuilder 的 SemanticSearchStrategy（对应配置 MEMORY_RELEVANT_TOP_K）。
    pub(crate) memory_relevant_top_k: usize,
    /// 语义检索最低相似度阈值，低于此分数的条目不进入上下文；0.0 表示不过滤（对应配置 MEMORY_SEMANTIC_MIN_SCORE）。
    pub(crate) memory_semantic_min_score: f32,
}

impl SyncAIHandler {
    /// 当用户仅 @ 提及机器人、未输入具体内容时，作为发给 AI 的默认“问题”提示，使 AI 简短打招呼并邀请用户提问。
    /// 与外部交互：作为 LlmClient 的 user 消息内容传入，由 LLM 生成友好回复。
    pub const DEFAULT_EMPTY_MENTION_QUESTION: &str =
        "用户只是 @ 了你，没有写具体问题。请简短友好地打招呼并邀请用户提问。";

    // ---------- Construction ----------

    pub fn new(
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        llm_client: Arc<OpenAILlmClient>,
        bot: Arc<dyn CoreBot>,
        repo: MessageRepository,
        memory_store: Arc<dyn MemoryStore>,
        recent_store: Option<Arc<dyn MemoryStore>>,
        embedding_service: Arc<dyn EmbeddingService>,
        use_streaming: bool,
        thinking_message: String,
        memory_recent_limit: usize,
        memory_relevant_top_k: usize,
        memory_semantic_min_score: f32,
    ) -> Self {
        Self {
            bot_username,
            llm_client,
            bot,
            repo,
            memory_store,
            recent_store,
            embedding_service,
            use_streaming,
            thinking_message,
            memory_recent_limit,
            memory_relevant_top_k,
            memory_semantic_min_score,
        }
    }

    async fn get_bot_username(&self) -> Option<String> {
        self.bot_username.read().await.clone()
    }

    // ---------- Question detection (reply-to-bot or @mention) ----------

    /// Returns true if the given text contains a @mention of the bot.
    /// External: none (pure function). Public for integration tests in `tests/`.
    pub fn is_bot_mentioned(&self, text: &str, bot_username: &str) -> bool {
        text.contains(&format!("@{}", bot_username))
    }

    /// Strips the bot @mention from text and returns the trimmed question. Used when processing @mention messages.
    /// External: none (pure function). Public for integration tests in `tests/`.
    pub fn extract_question(&self, text: &str, bot_username: &str) -> String {
        text.replace(&format!("@{}", bot_username), "")
            .trim()
            .to_string()
    }

    /// Resolves the user question: 回复机器人时用当前内容；@ 提及且问题非空时用提取后的内容；
    /// @ 提及但内容为空时用 DEFAULT_EMPTY_MENTION_QUESTION，使机器人仍会回复；否则 None。
    /// External: uses Message (dbot_core) fields only. Public for integration tests in `tests/`.
    pub fn get_question(&self, message: &Message, bot_username: Option<&str>) -> Option<String> {
        if message.reply_to_message_id.is_some() && message.reply_to_message_from_bot {
            return Some(message.content.clone());
        }
        if let Some(username) = bot_username {
            if self.is_bot_mentioned(&message.content, username) {
                let q = self.extract_question(&message.content, username);
                if !q.is_empty() {
                    return Some(q);
                }
                // 仅 @ 提及、无内容时仍触发回复，使用默认提示让 AI 打招呼并邀请用户提问
                return Some(Self::DEFAULT_EMPTY_MENTION_QUESTION.to_string());
            }
        }
        None
    }

    // ---------- Context & messages for AI ----------

    /// Logs error and its cause chain. First item with `first_msg`, rest with "Caused by".
    fn log_error_chain(e: &anyhow::Error, first_msg: &str) {
        for (i, cause) in e.chain().enumerate() {
            if i == 0 {
                error!(cause = %cause, "{}", first_msg);
            } else {
                error!(cause = %cause, "Caused by");
            }
        }
    }

    async fn build_memory_context(
        &self,
        user_id: &str,
        conversation_id: &str,
        question: &str,
    ) -> Option<Context> {
        let builder = ContextBuilder::new(self.memory_store.clone());
        let builder = if let Some(ref r) = self.recent_store {
            builder.with_recent_store(r.clone())
        } else {
            builder
        };
        let builder = builder
            .with_strategy(Box::new(RecentMessagesStrategy::new(self.memory_recent_limit)))
            .with_strategy(Box::new(SemanticSearchStrategy::new(
                self.memory_relevant_top_k,
                self.embedding_service.clone(),
                self.memory_semantic_min_score,
            )))
            .with_strategy(Box::new(UserPreferencesStrategy::new()))
            .with_token_limit(4096)
            .for_user(user_id)
            .for_conversation(conversation_id)
            .with_query(question);

        builder.build().await.map(Some).unwrap_or_else(|e| {
            Self::log_error_chain(&e, "Failed to build memory context");
            None
        })
    }

    /// Returns chat messages (system, user, assistant) for the AI request.
    ///
    /// When context is available, returns `context.to_messages(true, question)` which already
    /// contains system/user/assistant. When context is missing, returns a minimal user-only list.
    /// If the message is a reply to a bot message, the replied message content is prepended as
    /// assistant context so the AI understands what message the user is replying to.
    /// Callers do not construct messages; they use this method's return value directly.
    async fn build_messages_for_ai(
        &self,
        user_id: &str,
        conversation_id: &str,
        question: &str,
        reply_to_content: Option<&str>,
    ) -> Vec<prompt::ChatMessage> {
        let mut messages = match self.build_memory_context(user_id, conversation_id, question).await {
            Some(c) => c.to_messages(true, question),
            None => vec![prompt::ChatMessage::user(question)],
        };

        // 如果是回复机器人的消息，将被回复的内容作为助手消息插入到用户消息之前，
        // 让 AI 了解用户在回复哪条消息
        if let Some(replied_content) = reply_to_content {
            // 找到最后一条用户消息的位置，在其前面插入被回复的助手消息
            if let Some(last_user_idx) = messages.iter().rposition(|m| matches!(m.role, prompt::MessageRole::User)) {
                messages.insert(last_user_idx, prompt::ChatMessage::assistant(replied_content));
                info!(
                    replied_content_len = replied_content.len(),
                    "Inserted reply-to context as assistant message"
                );
            }
        }

        messages
    }

    // ---------- Sending & logging ----------

    async fn send_response_for_message(&self, message: &Message, response: &str) -> Result<()> {
        self.bot
            .send_message(&message.chat, response)
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

    /// Sends a fallback message to the user and returns `HandlerResponse::Stop`. Used on errors.
    async fn send_fallback_and_stop(
        &self,
        message: &Message,
        text: &str,
    ) -> Result<HandlerResponse> {
        let _ = self.send_response_for_message(message, text).await;
        Ok(HandlerResponse::Stop)
    }

    fn message_ids(message: &Message) -> (String, String) {
        (message.user.id.to_string(), message.chat.id.to_string())
    }

    // ---------- Processing: normal (non-streaming) ----------

    async fn process_normal(&self, message: &Message, question: &str) -> Result<HandlerResponse> {
        let (user_id, conversation_id) = Self::message_ids(message);
        let messages = self
            .build_messages_for_ai(
                &user_id,
                &conversation_id,
                question,
                message.reply_to_message_content.as_deref(),
            )
            .await;

        info!(
            message_count = messages.len(),
            question = %question,
            "Submitting to AI (non-streaming)"
        );
        log_messages_submitted_to_ai(&messages);

        let response = match self.llm_client.get_ai_response_with_messages(messages).await {
            Ok(r) => r,
            Err(e) => {
                Self::log_error_chain(&e, "Failed to get AI response");
                let err_str = e.to_string();
                if err_str.contains("401") || err_str.contains("令牌") {
                    error!(
                        "Hint: 401/令牌错误通常表示 OPENAI_API_KEY 已过期、无效，或与 OPENAI_BASE_URL 对应的服务不匹配，请检查 .env 配置"
                    );
                }
                return self.send_fallback_and_stop(message, MSG_REQUEST_FAILED).await;
            }
        };

        if let Err(e) = self.send_response_for_message(message, &response).await {
            error!(error = %e, "Failed to send AI response");
            return self.send_fallback_and_stop(message, MSG_SEND_FAILED).await;
        }
        Ok(HandlerResponse::Reply(response))
    }

    // ---------- Processing: streaming ----------

    async fn process_streaming(&self, message: &Message, question: &str) -> Result<HandlerResponse> {
        let (user_id, conversation_id) = Self::message_ids(message);
        debug!(
            user_id = %user_id,
            conversation_id = %conversation_id,
            question = %question,
            "Processing AI query (streaming)"
        );
        let messages = self
            .build_messages_for_ai(
                &user_id,
                &conversation_id,
                question,
                message.reply_to_message_content.as_deref(),
            )
            .await;

        info!(
            message_count = messages.len(),
            question = %question,
            "Submitting to AI (streaming)"
        );
        log_messages_submitted_to_ai(&messages);

        // Send "thinking" placeholder; on failure notify user and stop.
        let message_id = match self
            .bot
            .send_message_and_return_id(&message.chat, &self.thinking_message)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                error!(error = %e, "Failed to send thinking message");
                return self.send_fallback_and_stop(message, MSG_REQUEST_FAILED).await;
            }
        };

        let bot = self.bot.clone();
        let chat = message.chat.clone();
        let full_content = Arc::new(tokio::sync::Mutex::new(String::new()));

        // On each stream chunk: append to full_content and edit via Bot trait.
        match self
            .llm_client
            .get_ai_response_stream_with_messages(messages, |chunk| {
                let bot = bot.clone();
                let chat = chat.clone();
                let message_id = message_id.clone();
                let full_content = full_content.clone();
                async move {
                    if !chunk.content.is_empty() {
                        let mut content = full_content.lock().await;
                        content.push_str(&chunk.content);
                        bot.edit_message(&chat, &message_id, &*content)
                            .await
                            .map_err(|e| anyhow::anyhow!("Failed to edit message: {}", e))?;
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
                Self::log_error_chain(&e, "AI stream response failed");
                let _ = self
                    .bot
                    .edit_message(&message.chat, &message_id, MSG_STREAM_FAILED)
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
        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            "step: SyncAIHandler handle start"
        );

        let bot_username = self.get_bot_username().await;
        let question = match self.get_question(message, bot_username.as_deref()) {
            Some(q) => q,
            None => {
                if bot_username.is_none() && message.content.contains('@') {
                    info!(user_id = message.user.id, "step: SyncAIHandler bot_username not set yet; skip");
                } else {
                    info!(user_id = message.user.id, "step: SyncAIHandler not AI query (no reply-to-bot, no @mention), skip");
                }
                return Ok(HandlerResponse::Continue);
            }
        };
        info!(
            user_id = message.user.id,
            reply_to = ?message.reply_to_message_id,
            question = %question,
            "Processing AI query (reply-to-bot or @mention)"
        );

        if self.use_streaming {
            self.process_streaming(message, &question).await
        } else {
            self.process_normal(message, &question).await
        }
    }
}
