//! Telegram handler: when user replies to the bot or @mentions, runs ReAct agent with stream and edits the same message.
//!
//! Uses `run_chat_stream`, per-thread queue (messages are queued per chat and processed serially), and user-facing error messages.
//! Short-term memory is disabled: each turn uses only the current message (no conversation history).
//!
//! **Data flow:** `Handler::handle` → enqueue per chat → `process_queue_loop` consumes queue → `process_message` (send placeholder → spawn stream-edit → `run_chat_stream` → final edit).
//!
//! # Entry points (public API)
//!
//! - **[`AgentHandler`]** – Handler that runs the ReAct agent on reply-to-bot or @mention; implements [`Handler`](telegram_bot::Handler).
//! - **[`AgentHandler::new`]** – Constructs an `AgentHandler` with runner, bot, and placeholder message.
//! - **[`AgentHandler::get_question`]** – Returns the user question if the message should trigger the agent (reply-to-bot or @mention); otherwise `None`.
//! - **[`AgentHandler::thread_id`]** – Returns the thread ID (one per chat).
//! - **`Handler::handle`** (on `AgentHandler`) – Telegram entry: decides trigger, enqueues per-chat, returns immediately with `Continue` or `Stop`.

use crate::{run_chat_stream, ChatStreamResult, StreamUpdate, UserProfile};
use crate::ReactRunner;
use super::stream_edit::{format_reply_with_process_and_tools, is_message_not_modified_error, run_stream_edit_loop};
use async_trait::async_trait;
use std::sync::Arc;
use telegram_bot::mention;
use telegram_bot::{Bot, Chat, Handler, HandlerResponse, Message, Result};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument};

// ---------- User-facing messages (shown in Telegram) ----------
const MSG_SEND_FAILED: &str = "发送失败，请稍后再试。";
const MSG_PROCESSING_FAILED: &str = "处理时出错，请稍后再试。";
/// Shown when the agent completed (e.g. tool use) but returned no assistant text (e.g. only remember tool call).
const MSG_EMPTY_REPLY_FALLBACK: &str = "已处理。（本次无文字回复）";

// ---------- Type aliases (all user-visible strings are in the constants above) ----------

/// One item in the per-chat queue: the Telegram message and the extracted question text.
type QueuedItem = (Message, String);
/// Sender to the per-chat processing queue.
type QueueSender = mpsc::UnboundedSender<QueuedItem>;

// ---------- Handler (entry: AgentHandler, Handler::handle) ----------

/// **Entry point.** Handler that runs the ReAct agent on Telegram messages (reply-to-bot or @mention), streams the reply, and edits the same message.
/// Use with the Telegram bot framework; implements [`Handler`](telegram_bot::Handler). Incoming messages are queued per chat and processed serially.
pub struct AgentHandler {
    runner: Arc<ReactRunner>,
    bot: Arc<dyn Bot>,
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    placeholder_message: String,
    message_queues: dashmap::DashMap<String, QueueSender>,
}

impl AgentHandler {
    /// **Entry point.** Creates a new `AgentHandler` with the given runner, bot, and placeholder message.
    pub fn new(
        runner: Arc<ReactRunner>,
        bot: Arc<dyn Bot>,
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        placeholder_message: String,
    ) -> Self {
        Self {
            runner,
            bot,
            bot_username,
            placeholder_message,
            message_queues: dashmap::DashMap::new(),
        }
    }

    /// Returns the user question if the message triggers the agent (reply-to-bot or @mention); otherwise `None`. Used by [`ensure_trigger_question`](Self::ensure_trigger_question) and tests. Delegates to [`telegram_bot::mention::get_question`].
    pub fn get_question(&self, message: &Message, bot_username: Option<&str>) -> Option<String> {
        mention::get_question(message, bot_username, Some(mention::DEFAULT_EMPTY_MENTION_PROMPT))
    }

    /// **Entry point.** Thread ID for checkpoint: one conversation per chat (private or group). v1 uses chat_id.
    pub fn thread_id(message: &Message) -> String {
        message.chat.id.to_string()
    }

    /// If the message triggers the agent (reply-to-bot or @mention), returns `Some(question)`; otherwise `None`. Reads [`bot_username`](AgentHandler::bot_username) under lock.
    async fn ensure_trigger_question(&self, message: &Message) -> Option<String> {
        let bot_username = self.bot_username.read().await.clone();
        self.get_question(message, bot_username.as_deref())
    }

    fn user_profile_from_message(message: &Message) -> UserProfile {
        UserProfile {
            user_id: message.user.id.to_string(),
            first_name: message.user.first_name.clone(),
            last_name: message.user.last_name.clone(),
            username: message.user.username.clone(),
        }
    }

    fn get_or_create_queue(&self, thread_id: String) -> QueueSender {
        let (tx, rx) = mpsc::unbounded_channel::<QueuedItem>();
        let runner = self.runner.clone();
        let bot = self.bot.clone();
        let placeholder_message = self.placeholder_message.clone();
        tokio::spawn(Self::process_queue_loop(rx, runner, bot, placeholder_message, thread_id));
        tx
    }

    /// Consumes items from the per-chat queue and processes each with [`Self::process_message`].
    async fn process_queue_loop(
        mut rx: mpsc::UnboundedReceiver<QueuedItem>,
        runner: Arc<ReactRunner>,
        bot: Arc<dyn Bot>,
        placeholder_message: String,
        thread_id: String,
    ) {
        while let Some((message, question)) = rx.recv().await {
            info!(
                user_id = message.user.id,
                thread_id = %thread_id,
                "Processing queued message"
            );
            if let Err(e) = Self::process_message(
                &runner,
                &bot,
                &message,
                &question,
                &placeholder_message,
            )
            .await
            {
                error!(error = %e, user_id = message.user.id, "Failed to process queued message");
            }
        }
    }

    /// Sends the placeholder message and returns the message ID. On failure returns `Err`; the caller logs and notifies the user with [`MSG_SEND_FAILED`].
    async fn send_placeholder_message(
        bot: &Arc<dyn Bot>,
        chat: &Chat,
        placeholder_message: &str,
    ) -> Result<String> {
        let id = bot.send_message_and_return_id(chat, placeholder_message).await?;
        Ok(id)
    }

    /// Starts the stream-edit loop in a background task; returns the sender and the join handle. Caller must drop the sender and await the handle when done.
    fn spawn_stream_edit_task(
        bot: &Arc<dyn Bot>,
        chat: &Chat,
        message_id: &str,
    ) -> (mpsc::UnboundedSender<StreamUpdate>, JoinHandle<()>) {
        let (tx, rx) = mpsc::unbounded_channel::<StreamUpdate>();
        let bot_clone = bot.clone();
        let chat = chat.clone();
        let message_id = message_id.to_string();
        let handle = tokio::spawn(async move {
            run_stream_edit_loop(bot_clone, chat, message_id, rx).await
        });
        (tx, handle)
    }

    /// Picks the reply text to show: agent reply or [`MSG_EMPTY_REPLY_FALLBACK`] when reply is empty. Logs accordingly.
    fn pick_reply_text(result: &ChatStreamResult, thread_id: &str) -> String {
        if result.reply.trim().is_empty() {
            info!(
                thread_id = %thread_id,
                steps = ?result.steps,
                tools_used = ?result.tools_used,
                reply_len = result.reply.len(),
                "Agent completed with empty assistant reply; showing fallback message"
            );
            MSG_EMPTY_REPLY_FALLBACK.to_string()
        } else {
            info!(
                thread_id = %thread_id,
                reply_len = result.reply.len(),
                "run_chat_stream: success"
            );
            result.reply.clone()
        }
    }

    /// Edits the message to the final text. Logs only if the error is not "message is not modified" (best-effort edit).
    async fn apply_final_edit(
        bot: &Arc<dyn Bot>,
        chat: &Chat,
        message_id: &str,
        text: &str,
    ) {
        if let Err(e) = bot.edit_message(chat, message_id, text).await {
            if !is_message_not_modified_error(&e.to_string()) {
                error!(error = %e, "Failed to edit final message");
            }
        }
    }

    /// 1) Send placeholder; 2) spawn stream-edit loop; 3) run agent stream (no short-term memory); 4) apply final edit or error message.
    async fn process_message(
        runner: &Arc<ReactRunner>,
        bot: &Arc<dyn Bot>,
        message: &Message,
        question: &str,
        placeholder_message: &str,
    ) -> Result<HandlerResponse> {
        let thread_id = Self::thread_id(message);

        let message_id = match Self::send_placeholder_message(bot, &message.chat, placeholder_message).await {
            Ok(id) => id,
            Err(e) => {
                error!(error = %e, "Failed to send placeholder");
                let _ = bot.send_message(&message.chat, MSG_SEND_FAILED).await;
                return Ok(HandlerResponse::Stop);
            }
        };

        let (tx, edit_handle) = Self::spawn_stream_edit_task(bot, &message.chat, &message_id);

        let profile = Self::user_profile_from_message(message);
        info!(
            thread_id = %thread_id,
            question_len = question.len(),
            question_preview = %question.chars().take(60).collect::<String>(),
            "run_chat_stream: invoking"
        );
        let stream_result = run_chat_stream(
            runner.as_ref(),
            &thread_id,
            question,
            |update: StreamUpdate| {
                debug!(?update, "Stream update");
                let _ = tx.send(update);
            },
            Some(&profile),
            false,
        )
        .await;

        drop(tx);
        let _ = edit_handle.await;

        match stream_result {
            Ok(result) => {
                let reply_text = Self::pick_reply_text(&result, &thread_id);
                let text = format_reply_with_process_and_tools(&result.steps, &result.tools_used, &reply_text);
                Self::apply_final_edit(bot, &message.chat, &message_id, &text).await;
                Ok(HandlerResponse::Reply(text))
            }
            Err(e) => {
                error!(error = %e, "run_chat_stream failed");
                let _ = bot
                    .edit_message(&message.chat, &message_id, MSG_PROCESSING_FAILED)
                    .await;
                Ok(HandlerResponse::Stop)
            }
        }
    }
}

/// **Entry point.** Telegram framework calls this for each incoming message.
/// If the message triggers the agent (reply-to-bot or @mention), it is queued per chat and processed; returns `Continue` or `Stop` immediately.
#[async_trait]
impl Handler for AgentHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let question = match self.ensure_trigger_question(message).await {
            Some(q) => q,
            None => {
                info!(
                    user_id = message.user.id,
                    "AgentHandler: not a trigger (no reply-to-bot, no @mention), continue"
                );
                return Ok(HandlerResponse::Continue);
            }
        };

        let thread_id = Self::thread_id(message);
        info!(
            user_id = message.user.id,
            thread_id = %thread_id,
            "AgentHandler: queuing ReAct query"
        );

        let tx = self
            .message_queues
            .entry(thread_id.clone())
            .or_insert_with(|| self.get_or_create_queue(thread_id))
            .clone();

        if tx.send((message.clone(), question)).is_err() {
            error!(user_id = message.user.id, "Failed to send message to queue (receiver dropped)");
            return Ok(HandlerResponse::Stop);
        }

        Ok(HandlerResponse::Continue)
    }
}
