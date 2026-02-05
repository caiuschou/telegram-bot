//! Telegram handler: when user replies to the bot or @mentions, runs ReAct agent with stream and edits the same message.
//!
//! Uses `run_chat_stream`, per-thread queue (messages are queued per chat and processed serially), and user-facing error messages.
//! On receive, the user message is written to short-term memory via `append_user_message_into_checkpointer` before running the agent.

use crate::{append_user_message_into_checkpointer, run_chat_stream, StreamUpdate, UserProfile};
use crate::ReactRunner;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use telegram_bot::{Bot, Handler, HandlerResponse, Message, Result};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

const MSG_SEND_FAILED: &str = "发送失败，请稍后再试。";
const MSG_PROCESSING_FAILED: &str = "处理时出错，请稍后再试。";
/// Shown when the agent completed (e.g. tool use) but returned no assistant text (e.g. only remember tool call).
const MSG_EMPTY_REPLY_FALLBACK: &str = "已处理。（本次无文字回复）";
const DEFAULT_EMPTY_MENTION: &str =
    "The user only @mentioned you with no specific question. Please greet them briefly and invite them to ask.";

/// Builds the final message shown to the user: optional 【过程】, 【工具】, 【思考】, then the reply.
fn format_reply_with_process_and_tools(steps: &[String], tools_used: &[String], reply: &str) -> String {
    let mut parts = Vec::new();
    if !steps.is_empty() {
        parts.push(format!("【过程】{}", steps.join(" → ")));
    }
    if !tools_used.is_empty() {
        parts.push(format!("【工具】{}", tools_used.join(", ")));
    }
    if parts.is_empty() && reply.is_empty() {
        return String::new();
    }
    if !reply.is_empty() {
        parts.push(format!("【思考】\n\n{}", reply));
    } else if !parts.is_empty() {
        parts.push("【思考】".to_string());
    }
    parts.join("\n\n")
}

fn extract_retry_after_seconds(error: &str) -> Option<u64> {
    let pattern = "Retry after ";
    if let Some(start) = error.find(pattern) {
        let start = start + pattern.len();
        if let Some(end) = error[start..].find('s') {
            let seconds_str = &error[start..start + end];
            seconds_str.trim().parse().ok()
        } else {
            None
        }
    } else {
        None
    }
}

/// True when Telegram returns "message is not modified" (content unchanged); treat as success.
fn is_message_not_modified_error(error: &str) -> bool {
    error.contains("message is not modified") || error.contains("exactly the same")
}

/// Handler that runs the ReAct agent on Telegram messages (reply-to-bot or @mention), streams the reply, and edits the same message.
pub struct AgentHandler {
    runner: Arc<ReactRunner>,
    bot: Arc<dyn Bot>,
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    placeholder_message: String,
    db_path: PathBuf,
    message_queues: dashmap::DashMap<String, tokio::sync::mpsc::UnboundedSender<(Message, String)>>,
}

impl AgentHandler {
    /// Creates a new AgentHandler with the given runner, bot, config, and checkpoint DB path (used to write user messages to short-term memory on receive).
    pub fn new(
        runner: Arc<ReactRunner>,
        bot: Arc<dyn Bot>,
        bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
        placeholder_message: String,
        db_path: PathBuf,
    ) -> Self {
        Self {
            runner,
            bot,
            bot_username,
            placeholder_message,
            db_path,
            message_queues: dashmap::DashMap::new(),
        }
    }

    fn is_bot_mentioned(&self, text: &str, bot_username: &str) -> bool {
        text.contains(&format!("@{}", bot_username))
    }

    fn extract_question(&self, text: &str, bot_username: &str) -> String {
        text.replace(&format!("@{}", bot_username), "")
            .trim()
            .to_string()
    }

    /// Returns the user question if the message should trigger the agent (reply-to-bot or @mention); otherwise None.
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
                return Some(DEFAULT_EMPTY_MENTION.to_string());
            }
        }
        None
    }

    /// Thread ID for checkpoint: one conversation per chat (private or group). v1 uses chat_id.
    pub fn thread_id(message: &Message) -> String {
        message.chat.id.to_string()
    }

    fn user_profile_from_message(message: &Message) -> UserProfile {
        UserProfile {
            user_id: message.user.id.to_string(),
            first_name: message.user.first_name.clone(),
            last_name: message.user.last_name.clone(),
            username: message.user.username.clone(),
        }
    }

    fn get_or_create_queue(&self, thread_id: String) -> tokio::sync::mpsc::UnboundedSender<(Message, String)> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(Message, String)>();

        let runner = self.runner.clone();
        let bot = self.bot.clone();
        let placeholder_message = self.placeholder_message.clone();
        let db_path = self.db_path.clone();

        tokio::spawn(async move {
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
                    &db_path,
                ).await {
                    error!(error = %e, user_id = message.user.id, "Failed to process queued message");
                }
            }
        });

        tx
    }

    async fn process_message(
        runner: &Arc<ReactRunner>,
        bot: &Arc<dyn Bot>,
        message: &Message,
        question: &str,
        placeholder_message: &str,
        db_path: &std::path::Path,
    ) -> Result<HandlerResponse> {
        let thread_id = Self::thread_id(message);

        let message_id = match bot
            .send_message_and_return_id(&message.chat, placeholder_message)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                error!(error = %e, "Failed to send placeholder");
                let _ = bot.send_message(&message.chat, MSG_SEND_FAILED).await;
                return Ok(HandlerResponse::Stop);
            }
        };

        let (tx, mut rx) = mpsc::unbounded_channel::<StreamUpdate>();
        let bot_clone = bot.clone();
        let chat = message.chat.clone();
        let message_id_edit = message_id.clone();
        const EDIT_CHUNK_SIZE: usize = 50;
        const MAX_EDIT_DELAY_SECS: u64 = 2;

        let edit_handle = tokio::spawn(async move {
            let mut steps: Vec<String> = Vec::new();
            let mut tools_used: Vec<String> = Vec::new();
            let mut content = String::new();
            let mut buffer = String::new();
            let mut last_edit = Instant::now();

            let do_edit = |steps: &[String], tools: &[String], content: &str| {
                format_reply_with_process_and_tools(steps, tools, content)
            };

            loop {
                let timeout = if buffer.is_empty() {
                    None
                } else {
                    let elapsed = last_edit.elapsed().as_secs();
                    Some(MAX_EDIT_DELAY_SECS.saturating_sub(elapsed))
                };

                let (recv_finished, opt_update) = match timeout {
                    Some(secs) => {
                        tokio::select! {
                            result = rx.recv() => (result.is_none(), result),
                            _ = tokio::time::sleep(std::time::Duration::from_secs(secs)) => {
                                if !buffer.is_empty() {
                                    content.push_str(&buffer);
                                    buffer.clear();
                                    let text = do_edit(&steps, &tools_used, &content);
                                    loop {
                                        match bot_clone.edit_message(&chat, &message_id_edit, &text).await {
                                            Ok(_) => {
                                                last_edit = Instant::now();
                                                break;
                                            }
                                            Err(e) => {
                                                let error_str = e.to_string();
                                                if is_message_not_modified_error(&error_str) {
                                                    last_edit = Instant::now();
                                                    break;
                                                }
                                                if let Some(retry_secs) = extract_retry_after_seconds(&error_str) {
                                                    error!(error = %e, "Failed to edit message, retrying after {}s", retry_secs);
                                                    tokio::time::sleep(std::time::Duration::from_secs(retry_secs)).await;
                                                } else {
                                                    error!(error = %e, "Failed to edit message");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                                (false, None)
                            }
                        }
                    }
                    None => {
                        match rx.recv().await {
                            Some(up) => (false, Some(up)),
                            None => break,
                        }
                    }
                };

                if recv_finished {
                    break;
                }

                if let Some(update) = opt_update {
                    match update {
                        StreamUpdate::Chunk(s) | StreamUpdate::ThinkChunk(s) => {
                            buffer.push_str(&s);
                            if buffer.len() >= EDIT_CHUNK_SIZE {
                                content.push_str(&buffer);
                                buffer.clear();
                                let text = do_edit(&steps, &tools_used, &content);
                                loop {
                                    match bot_clone.edit_message(&chat, &message_id_edit, &text).await {
                                        Ok(_) => {
                                            last_edit = Instant::now();
                                            break;
                                        }
                                        Err(e) => {
                                            let error_str = e.to_string();
                                            if is_message_not_modified_error(&error_str) {
                                                last_edit = Instant::now();
                                                break;
                                            }
                                            if let Some(secs) = extract_retry_after_seconds(&error_str) {
                                                error!(error = %e, "Failed to edit message, retrying after {}s", secs);
                                                tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                                            } else {
                                                error!(error = %e, "Failed to edit message");
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        StreamUpdate::Steps(s) => {
                            steps = s;
                            let text = do_edit(&steps, &tools_used, &content);
                            loop {
                                match bot_clone.edit_message(&chat, &message_id_edit, &text).await {
                                    Ok(_) => {
                                        last_edit = Instant::now();
                                        break;
                                    }
                                    Err(e) => {
                                        let error_str = e.to_string();
                                        if is_message_not_modified_error(&error_str) {
                                            last_edit = Instant::now();
                                            break;
                                        }
                                        if let Some(secs) = extract_retry_after_seconds(&error_str) {
                                            error!(error = %e, "Failed to edit message, retrying after {}s", secs);
                                            tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                                        } else {
                                            error!(error = %e, "Failed to edit message");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        StreamUpdate::Tools(t) => {
                            tools_used = t;
                            let text = do_edit(&steps, &tools_used, &content);
                            loop {
                                match bot_clone.edit_message(&chat, &message_id_edit, &text).await {
                                    Ok(_) => {
                                        last_edit = Instant::now();
                                        break;
                                    }
                                    Err(e) => {
                                        let error_str = e.to_string();
                                        if is_message_not_modified_error(&error_str) {
                                            last_edit = Instant::now();
                                            break;
                                        }
                                        if let Some(secs) = extract_retry_after_seconds(&error_str) {
                                            error!(error = %e, "Failed to edit message, retrying after {}s", secs);
                                            tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
                                        } else {
                                            error!(error = %e, "Failed to edit message");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !buffer.is_empty() {
                content.push_str(&buffer);
            }
            let text = do_edit(&steps, &tools_used, &content);
            loop {
                match bot_clone.edit_message(&chat, &message_id_edit, &text).await {
                    Ok(_) => break,
                    Err(e) => {
                        let error_str = e.to_string();
                        if is_message_not_modified_error(&error_str) {
                            break;
                        }
                        if let Some(retry_secs) = extract_retry_after_seconds(&error_str) {
                            error!(error = %e, "Failed to edit message, retrying after {}s", retry_secs);
                            tokio::time::sleep(std::time::Duration::from_secs(retry_secs)).await;
                        } else {
                            error!(error = %e, "Failed to edit message");
                            break;
                        }
                    }
                }
            }
        });

        if let Err(e) = append_user_message_into_checkpointer(db_path, &thread_id, question).await {
            error!(error = %e, thread_id = %thread_id, "Failed to append user message to short-term memory");
        }
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
            true,
        )
        .await;

        drop(tx);
        let _ = edit_handle.await;

        match stream_result {
            Ok(result) => {
                let reply_text = if result.reply.trim().is_empty() {
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
                    result.reply
                };
                let text = format_reply_with_process_and_tools(&result.steps, &result.tools_used, &reply_text);
                if let Err(e) = bot.edit_message(&message.chat, &message_id, &text).await {
                    if !is_message_not_modified_error(&e.to_string()) {
                        error!(error = %e, "Failed to edit final message");
                    }
                }
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

#[async_trait]
impl Handler for AgentHandler {
    #[instrument(skip(self, message))]
    async fn handle(&self, message: &Message) -> Result<HandlerResponse> {
        let bot_username = self.bot_username.read().await.clone();
        let question = match self.get_question(message, bot_username.as_deref()) {
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

        let tx = self.message_queues.entry(thread_id.clone())
            .or_insert_with(|| self.get_or_create_queue(thread_id))
            .clone();

        if let Err(_) = tx.send((message.clone(), question)) {
            error!(user_id = message.user.id, "Failed to send message to queue (receiver dropped)");
            return Ok(HandlerResponse::Stop);
        }

        Ok(HandlerResponse::Continue)
    }
}
