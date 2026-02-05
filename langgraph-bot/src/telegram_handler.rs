//! Telegram handler: when user replies to the bot or @mentions, runs ReAct agent with stream and edits the same message.
//!
//! Uses `run_chat_stream`, same-thread serialization (one request per thread at a time), and user-facing error messages.

use crate::{run_chat_stream, UserProfile};
use crate::ReactRunner;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use telegram_bot::{Bot, Handler, HandlerResponse, Message, Result};
use tokio::sync::mpsc;
use tracing::{debug, error, info, instrument};

        const MSG_SEND_FAILED: &str = "发送失败，请稍后再试。";
const MSG_PROCESSING_FAILED: &str = "处理时出错，请稍后再试。";
const MSG_BUSY: &str = "上一条还在处理中，请稍候。";
const DEFAULT_EMPTY_MENTION: &str =
    "The user only @mentioned you with no specific question. Please greet them briefly and invite them to ask.";

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

/// Guard that clears the busy flag for a thread when dropped.
struct ThreadBusyGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for ThreadBusyGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

/// Handler that runs the ReAct agent on Telegram messages (reply-to-bot or @mention), streams the reply, and edits the same message.
pub struct AgentHandler {
    runner: Arc<ReactRunner>,
    bot: Arc<dyn Bot>,
    bot_username: Arc<tokio::sync::RwLock<Option<String>>>,
    placeholder_message: String,
    thread_busy: dashmap::DashMap<String, Arc<AtomicBool>>,
}

impl AgentHandler {
    /// Creates a new AgentHandler with the given runner, bot, and config.
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
            thread_busy: dashmap::DashMap::new(),
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
        let flag = self
            .thread_busy
            .entry(thread_id.clone())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone();
        if flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            let _ = self.bot.send_message(&message.chat, MSG_BUSY).await;
            return Ok(HandlerResponse::Stop);
        }
        let _guard = ThreadBusyGuard { flag };

        info!(
            user_id = message.user.id,
            thread_id = %thread_id,
            "AgentHandler: processing ReAct query"
        );

        let message_id = match self
            .bot
            .send_message_and_return_id(&message.chat, &self.placeholder_message)
            .await
        {
            Ok(id) => id,
            Err(e) => {
                error!(error = %e, "Failed to send placeholder");
                let _ = self.bot.send_message(&message.chat, MSG_SEND_FAILED).await;
                return Ok(HandlerResponse::Stop);
            }
        };

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        let bot = self.bot.clone();
        let chat = message.chat.clone();
        let message_id_edit = message_id.clone();
        const EDIT_CHUNK_SIZE: usize = 50;
        const MAX_EDIT_DELAY_SECS: u64 = 2;

        let edit_handle = tokio::spawn(async move {
            let mut content = String::new();
            let mut buffer = String::new();
            let mut last_edit = Instant::now();

            loop {
                let timeout = if buffer.is_empty() {
                    None
                } else {
                    let elapsed = last_edit.elapsed().as_secs();
                    Some(MAX_EDIT_DELAY_SECS.saturating_sub(elapsed))
                };

                match timeout {
                    Some(secs) => {
                        tokio::select! {
                            result = rx.recv() => {
                                match result {
                                Some(chunk) => {
                                    buffer.push_str(&chunk);
                                    if buffer.len() >= EDIT_CHUNK_SIZE {
                                        content.push_str(&buffer);
                                        buffer.clear();
                                        loop {
                                            match bot.edit_message(&chat, &message_id_edit, &content).await {
                                                Ok(_) => {
                                                    last_edit = Instant::now();
                                                    break;
                                                }
                                                Err(e) => {
                                                    let error_str = e.to_string();
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
                                    None => break,
                                }
                            }
                            _ = tokio::time::sleep(std::time::Duration::from_secs(secs)) => {
                                if !buffer.is_empty() {
                                    content.push_str(&buffer);
                                    buffer.clear();
                                    loop {
                                        match bot.edit_message(&chat, &message_id_edit, &content).await {
                                            Ok(_) => {
                                                last_edit = Instant::now();
                                                break;
                                            }
                                            Err(e) => {
                                                let error_str = e.to_string();
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
                            }
                        }
                    }
                    None => {
                        match rx.recv().await {
                            Some(chunk) => {
                                buffer.push_str(&chunk);
                                if buffer.len() >= EDIT_CHUNK_SIZE {
                                    content.push_str(&buffer);
                                    buffer.clear();
                                    loop {
                                        match bot.edit_message(&chat, &message_id_edit, &content).await {
                                            Ok(_) => {
                                                last_edit = Instant::now();
                                                break;
                                            }
                                            Err(e) => {
                                                let error_str = e.to_string();
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
                            }
                            None => break,
                        }
                    }
                }
            }

            if !buffer.is_empty() {
                content.push_str(&buffer);
                loop {
                    match bot.edit_message(&chat, &message_id_edit, &content).await {
                        Ok(_) => break,
                        Err(e) => {
                            let error_str = e.to_string();
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
        });

        let profile = Self::user_profile_from_message(message);
        let stream_result = run_chat_stream(
            self.runner.as_ref(),
            &thread_id,
            &question,
            |chunk: &str| {
                debug!(chunk_len = chunk.len(), preview = %chunk.chars().take(50).collect::<String>(), "Received stream chunk");
                let _ = tx.send(chunk.to_string());
            },
            Some(&profile),
        )
        .await;

        drop(tx);
        let _ = edit_handle.await;

        match stream_result {
            Ok(final_reply) => {
                let _ = self
                    .bot
                    .edit_message(&message.chat, &message_id, &final_reply)
                    .await;
                Ok(HandlerResponse::Reply(final_reply))
            }
            Err(e) => {
                error!(error = %e, "run_chat_stream failed");
                let _ = self
                    .bot
                    .edit_message(&message.chat, &message_id, MSG_PROCESSING_FAILED)
                    .await;
                Ok(HandlerResponse::Stop)
            }
        }
    }
}
