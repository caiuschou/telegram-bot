//! Inline LLM handler: runs in the handler chain, calls LLM and returns `HandlerResponse::Reply(text)` so later handlers (e.g. memory) can save the reply in `after()`.

use llm_client::{LlmClient, StreamChunk, StreamChunkCallback};
use telegram_bot::mention;
use async_trait::async_trait;
use telegram_bot::{Bot as CoreBot, Handler, HandlerResponse, Message, Result};
use telegram_bot::embedding::EmbeddingService;
use telegram_bot::memory::{
    Context, ContextBuilder, MemoryStore, RecentMessagesStrategy, SemanticSearchStrategy,
    UserPreferencesStrategy,
};
use prompt::ChatMessage;
use std::sync::Arc;
use std::time::Instant;
use telegram_bot::storage::MessageRepository;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument};

// --- User-facing fallback messages (sent to Telegram on errors) ---
const MSG_SEND_FAILED: &str = "Sorry, something went wrong while sending the reply.";
const MSG_REQUEST_FAILED: &str = "Sorry, something went wrong processing your request. Please try again later.";
const MSG_STREAM_FAILED: &str = "Sorry, LLM response failed.";

/// Logs the exact messages submitted to the LLM (role + full content) for debugging.
fn log_messages_submitted_to_llm(messages: &[ChatMessage]) {
    info!(count = messages.len(), "submit_to_llm: messages submitted to LLM");
    for (i, m) in messages.iter().enumerate() {
        info!(
            index = i,
            role = ?m.role,
            content = %m.content,
            "submit_to_llm message"
        );
    }
}

/// Inline LLM handler: when the message is an LLM query (user replies to the bot's message, or @mentions the bot), builds context, calls the LLM, sends the reply to Telegram, and returns `HandlerResponse::Reply(response_text)` so later handlers can persist it in `after()` (e.g. memory handler).
///
/// **External interactions:** Bot trait (send/edit), MessageRepository (log), MemoryStore (context build), EmbeddingService (semantic search), LlmClient (LLM).
#[derive(Clone)]
pub struct InlineLLMHandler {
    pub(crate) bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    pub(crate) llm_client: Arc<dyn LlmClient>,
    pub(crate) bot: Arc<dyn CoreBot>,
    pub(crate) repo: MessageRepository,
    pub(crate) memory_store: Arc<dyn MemoryStore>,
    /// When set, RecentMessagesStrategy and UserPreferencesStrategy use this store (e.g. SQLite); semantic search still uses `memory_store`.
    pub(crate) recent_store: Option<Arc<dyn MemoryStore>>,
    pub(crate) embedding_service: Arc<dyn EmbeddingService>,
    pub(crate) use_streaming: bool,
    pub(crate) thinking_message: String,
    /// Max number of recent messages for ContextBuilder's RecentMessagesStrategy (config MEMORY_RECENT_LIMIT).
    pub(crate) memory_recent_limit: usize,
    /// Top-K for semantic search in ContextBuilder's SemanticSearchStrategy (config MEMORY_RELEVANT_TOP_K).
    pub(crate) memory_relevant_top_k: usize,
    /// Min similarity score for semantic results; entries below this are excluded from context; 0.0 = no filter (config MEMORY_SEMANTIC_MIN_SCORE).
    pub(crate) memory_semantic_min_score: f32,
    /// Min interval (seconds) between edits of the same message when streaming; limits Telegram edit rate (config TELEGRAM_EDIT_INTERVAL_SECS, default 5).
    pub(crate) edit_interval_secs: u64,
}

impl InlineLLMHandler {
    // ---------- Construction ----------

    /// Builds an InlineLLMHandler with the given dependencies and config (limits, streaming, edit interval).
    pub fn new(
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        llm_client: Arc<dyn LlmClient>,
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
        edit_interval_secs: u64,
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
            edit_interval_secs,
        }
    }

    async fn get_bot_username(&self) -> Option<String> {
        self.bot_username.read().await.clone()
    }

    // ---------- Question detection (reply-to-bot or @mention) ----------

    /// Returns true if the given text contains a @mention of the bot.
    /// Public for integration tests in `tests/`. Delegates to [`telegram_bot::mention::is_bot_mentioned`].
    pub fn is_bot_mentioned(&self, text: &str, bot_username: &str) -> bool {
        mention::is_bot_mentioned(text, bot_username)
    }

    /// Strips the bot @mention from text and returns the trimmed question. Used when processing @mention messages.
    /// Public for integration tests in `tests/`. Delegates to [`telegram_bot::mention::extract_question`].
    pub fn extract_question(&self, text: &str, bot_username: &str) -> String {
        mention::extract_question(text, bot_username)
    }

    /// Resolves the user question: when replying to bot use current content; when @mention with non-empty text use extracted content;
    /// when @mention but empty use [`mention::DEFAULT_EMPTY_MENTION_PROMPT`] so bot still replies; otherwise None.
    /// External: uses Message (telegram_bot) and [`telegram_bot::mention::get_question`]. Public for integration tests in `tests/`.
    pub fn get_question(&self, message: &Message, bot_username: Option<&str>) -> Option<String> {
        mention::get_question(message, bot_username, Some(mention::DEFAULT_EMPTY_MENTION_PROMPT))
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

    /// Returns chat messages (system, user, assistant) for the LLM request.
    async fn build_messages_for_llm(
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

        if let Some(replied_content) = reply_to_content {
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
                telegram_bot::DbotError::Bot(e.to_string())
            })?;
        self.log_llm_response_for_message(message, response).await?;
        info!(user_id = message.user.id, "LLM response sent");
        Ok(())
    }

    async fn log_llm_response_for_message(&self, message: &Message, response: &str) -> Result<()> {
        let record = telegram_bot::storage::MessageRecord::new(
            message.user.id,
            message.chat.id,
            None,
            None,
            None,
            "llm_response".to_string(),
            response.to_string(),
            "sent".to_string(),
        );
        self.repo
            .save(&record)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to save LLM response");
                telegram_bot::DbotError::Database(e.to_string())
            })?;
        Ok(())
    }

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

    async fn process_normal(&self, message: &Message, question: &str) -> Result<HandlerResponse> {
        let (user_id, conversation_id) = Self::message_ids(message);
        let messages = self
            .build_messages_for_llm(
                &user_id,
                &conversation_id,
                question,
                message.reply_to_message_content.as_deref(),
            )
            .await;

        info!(
            message_count = messages.len(),
            question = %question,
            "Submitting to LLM (non-streaming)"
        );
        log_messages_submitted_to_llm(&messages);

        let response = match self.llm_client.get_llm_response_with_messages(messages).await {
            Ok(r) => r,
            Err(e) => {
                Self::log_error_chain(&e, "Failed to get LLM response");
                let err_str = e.to_string();
                if err_str.contains("401") || err_str.contains("token") || err_str.contains("Token") {
                    error!(
                        "Hint: 401/token errors usually mean OPENAI_API_KEY is expired, invalid, or does not match OPENAI_BASE_URL; check .env"
                    );
                }
                return self.send_fallback_and_stop(message, MSG_REQUEST_FAILED).await;
            }
        };

        if let Err(e) = self.send_response_for_message(message, &response).await {
            error!(error = %e, "Failed to send LLM response");
            return self.send_fallback_and_stop(message, MSG_SEND_FAILED).await;
        }
        Ok(HandlerResponse::Reply(response))
    }

    async fn process_streaming(&self, message: &Message, question: &str) -> Result<HandlerResponse> {
        let (user_id, conversation_id) = Self::message_ids(message);
        debug!(
            user_id = %user_id,
            conversation_id = %conversation_id,
            question = %question,
            "Processing LLM query (streaming)"
        );
        let messages = self
            .build_messages_for_llm(
                &user_id,
                &conversation_id,
                question,
                message.reply_to_message_content.as_deref(),
            )
            .await;

        info!(
            message_count = messages.len(),
            question = %question,
            "Submitting to LLM (streaming)"
        );
        log_messages_submitted_to_llm(&messages);

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
        let message_id_for_callback = message_id.clone();
        let full_content = Arc::new(tokio::sync::Mutex::new(String::new()));
        let edit_interval_secs = self.edit_interval_secs;
        let last_edit = Arc::new(tokio::sync::Mutex::new(None::<Instant>));

        let mut stream_callback: Box<StreamChunkCallback> = Box::new(move |chunk: StreamChunk| {
            let bot = bot.clone();
            let chat = chat.clone();
            let message_id = message_id_for_callback.clone();
            let full_content = full_content.clone();
            let last_edit = last_edit.clone();
            Box::pin(async move {
                if !chunk.content.is_empty() {
                    if edit_interval_secs > 0 {
                        let last = last_edit.lock().await;
                        if let Some(prev) = *last {
                            let elapsed = prev.elapsed();
                            let interval = std::time::Duration::from_secs(edit_interval_secs);
                            if elapsed < interval {
                                drop(last);
                                sleep(interval - elapsed).await;
                            }
                        }
                    }
                    let mut content = full_content.lock().await;
                    content.push_str(&chunk.content);
                    bot.edit_message(&chat, &message_id, &*content)
                        .await
                        .map_err(|e| anyhow::anyhow!("Failed to edit message: {}", e))?;
                    if edit_interval_secs > 0 {
                        *last_edit.lock().await = Some(Instant::now());
                    }
                }
                Ok(())
            })
        });
        match self
            .llm_client
            .get_llm_response_stream_with_messages(messages, stream_callback.as_mut())
            .await
        {
            Ok(full_response) => {
                let _ = self.log_llm_response_for_message(message, &full_response).await;
                Ok(HandlerResponse::Reply(full_response))
            }
            Err(e) => {
                Self::log_error_chain(&e, "LLM stream response failed");
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
impl Handler for InlineLLMHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        info!(
            user_id = message.user.id,
            chat_id = message.chat.id,
            "step: InlineLLMHandler handle start"
        );

        let bot_username = self.get_bot_username().await;
        let question = match self.get_question(message, bot_username.as_deref()) {
            Some(q) => q,
            None => {
                if bot_username.is_none() && message.content.contains('@') {
                    info!(user_id = message.user.id, "step: InlineLLMHandler bot_username not set yet; skip");
                } else {
                    info!(user_id = message.user.id, "step: InlineLLMHandler not LLM query (no reply-to-bot, no @mention), skip");
                }
                return Ok(HandlerResponse::Continue);
            }
        };
        info!(
            user_id = message.user.id,
            reply_to = ?message.reply_to_message_id,
            question = %question,
            "Processing LLM query (reply-to-bot or @mention)"
        );

        if self.use_streaming {
            self.process_streaming(message, &question).await
        } else {
            self.process_normal(message, &question).await
        }
    }
}
