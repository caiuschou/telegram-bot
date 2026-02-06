//! Stream-edit loop: consumes `StreamUpdate`s from a channel and edits a Telegram message in place.
//!
//! Used by [`super::handler::AgentHandler`] via `tokio::spawn(run_stream_edit_loop(...))`.
//!
//! # Entry points
//!
//! - **[`run_stream_edit_loop`]** – Main loop; buffers content/steps/tools and edits periodically.
//! - **[`format_reply_with_process_and_tools`]** – Builds the final message (【过程】, 【工具】, 【思考】).
//! - **[`is_message_not_modified_error`]** – True when Telegram returns "message is not modified"; treat as success.

use crate::StreamUpdate;
use std::sync::Arc;
use std::time::Instant;
use telegram_bot::{Bot, Chat};
use tokio::sync::mpsc;
use tracing::error;

// ---------- Tuning constants ----------

/// Chunk size in characters before flushing buffer to Telegram.
const EDIT_CHUNK_SIZE: usize = 50;
/// Max delay (seconds) before flushing buffered content even if chunk size not reached.
const MAX_EDIT_DELAY_SECS: u64 = 2;

type BotRef = Arc<dyn Bot>;

/// Identifies which Telegram message we are editing; bundles bot, chat, and message ID.
struct EditTarget<'a> {
    bot: &'a BotRef,
    chat: &'a Chat,
    message_id: &'a str,
}

// ---------- Formatting ----------

/// Builds the final message shown to the user: optional 【过程】, 【工具】, 【思考】, then the reply.
pub fn format_reply_with_process_and_tools(
    steps: &[String],
    tools_used: &[String],
    reply: &str,
) -> String {
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

// ---------- Retry / Rate limit ----------

/// True when Telegram returns "message is not modified" (content unchanged); treat as success.
pub fn is_message_not_modified_error(error: &str) -> bool {
    error.contains("message is not modified") || error.contains("exactly the same")
}

/// Parses "Retry after Ns" from Telegram API error string; returns `Some(seconds)` for rate-limit retry.
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

/// Edits the Telegram message with `text`, retrying on rate-limit (Retry-After) and treating
/// "message is not modified" as success.
async fn edit_message_with_retry(target: &EditTarget<'_>, text: &str) {
    loop {
        match target.bot.edit_message(target.chat, target.message_id, text).await {
            Ok(_) => break,
            Err(e) => {
                let error_str = e.to_string();
                if is_message_not_modified_error(&error_str) {
                    break;
                }
                if let Some(retry_secs) = extract_retry_after_seconds(&error_str) {
                    error!(
                        error = %e,
                        "Failed to edit message, retrying after {}s", retry_secs
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(retry_secs)).await;
                } else {
                    error!(error = %e, "Failed to edit message");
                    break;
                }
            }
        }
    }
}

/// Builds text from steps/tools/content, sends edit with retry, and updates `last_edit`.
async fn send_edit(
    target: &EditTarget<'_>,
    steps: &[String],
    tools_used: &[String],
    content: &str,
    last_edit: &mut Instant,
) {
    let text = format_reply_with_process_and_tools(steps, tools_used, content);
    edit_message_with_retry(target, &text).await;
    *last_edit = Instant::now();
}

// ---------- Channel recv with timeout ----------

/// Result of receiving from a channel with an optional timeout: either an item, channel closed, or timeout.
#[derive(Debug)]
enum RecvWithTimeoutResult<T> {
    Item(T),
    Closed,
    Timeout,
}

/// Receives one item from `rx`. If `timeout_secs` is `Some(secs)`, races with a sleep and returns
/// [`RecvWithTimeoutResult::Timeout`] when the sleep wins.
async fn recv_with_timeout<T>(
    rx: &mut mpsc::UnboundedReceiver<T>,
    timeout_secs: Option<u64>,
) -> RecvWithTimeoutResult<T> {
    match timeout_secs {
        None => match rx.recv().await {
            Some(t) => RecvWithTimeoutResult::Item(t),
            None => RecvWithTimeoutResult::Closed,
        },
        Some(secs) => {
            tokio::select! {
                result = rx.recv() => match result {
                    Some(t) => RecvWithTimeoutResult::Item(t),
                    None => RecvWithTimeoutResult::Closed,
                },
                _ = tokio::time::sleep(std::time::Duration::from_secs(secs)) => RecvWithTimeoutResult::Timeout,
            }
        }
    }
}

/// Returns the timeout in seconds for the next recv when we have buffered content: `None` when buffer is empty,
/// otherwise `Some(remaining_secs)` so we flush by max delay.
fn stream_edit_timeout_secs(buffer: &str, last_edit: Instant) -> Option<u64> {
    if buffer.is_empty() {
        return None;
    }
    let elapsed = last_edit.elapsed().as_secs();
    Some(MAX_EDIT_DELAY_SECS.saturating_sub(elapsed))
}

// ---------- Stream-edit state & loop ----------

/// Mutable state for the stream-edit loop: buffers content/steps/tools and drives periodic edits.
struct StreamEditState {
    steps: Vec<String>,
    tools_used: Vec<String>,
    content: String,
    buffer: String,
    last_edit: Instant,
}

impl StreamEditState {
    fn new() -> Self {
        Self {
            steps: Vec::new(),
            tools_used: Vec::new(),
            content: String::new(),
            buffer: String::new(),
            last_edit: Instant::now(),
        }
    }

    /// Timeout in seconds for the next recv; used so we flush by max delay when buffer is non-empty.
    fn timeout_secs(&self) -> Option<u64> {
        stream_edit_timeout_secs(&self.buffer, self.last_edit)
    }

    /// Flushes `buffer` into `content`, then sends edit. No-op when `buffer` is empty.
    async fn flush_buffer_and_send(&mut self, target: &EditTarget<'_>) {
        if self.buffer.is_empty() {
            return;
        }
        self.content.push_str(&self.buffer);
        self.buffer.clear();
        send_edit(
            target,
            &self.steps,
            &self.tools_used,
            &self.content,
            &mut self.last_edit,
        )
        .await;
    }

    /// Handles one [`RecvWithTimeoutResult`]. Returns `false` when the channel is closed (caller should break).
    async fn process_recv_result(
        &mut self,
        result: RecvWithTimeoutResult<StreamUpdate>,
        target: &EditTarget<'_>,
    ) -> bool {
        match result {
            RecvWithTimeoutResult::Closed => false,
            RecvWithTimeoutResult::Timeout => {
                self.flush_buffer_and_send(target).await;
                true
            }
            RecvWithTimeoutResult::Item(update) => {
                self.apply_stream_update(update, target).await;
                true
            }
        }
    }

    /// Handles Chunk or ThinkChunk: appends to buffer, flushes when chunk size reached.
    async fn handle_chunk_or_think(&mut self, s: String, target: &EditTarget<'_>) {
        self.buffer.push_str(&s);
        if self.buffer.len() >= EDIT_CHUNK_SIZE {
            self.flush_buffer_and_send(target).await;
        }
    }

    /// Handles Steps update: replaces steps and sends immediate edit.
    async fn handle_steps(&mut self, s: Vec<String>, target: &EditTarget<'_>) {
        self.steps = s;
        send_edit(
            target,
            &self.steps,
            &self.tools_used,
            &self.content,
            &mut self.last_edit,
        )
        .await;
    }

    /// Handles Tools update: replaces tools_used and sends immediate edit.
    async fn handle_tools(&mut self, t: Vec<String>, target: &EditTarget<'_>) {
        self.tools_used = t;
        send_edit(
            target,
            &self.steps,
            &self.tools_used,
            &self.content,
            &mut self.last_edit,
        )
        .await;
    }

    /// Applies a single stream update (chunk, steps, or tools) and may flush/edit.
    async fn apply_stream_update(&mut self, update: StreamUpdate, target: &EditTarget<'_>) {
        match update {
            StreamUpdate::Chunk(s) | StreamUpdate::ThinkChunk(s) => {
                self.handle_chunk_or_think(s, target).await;
            }
            StreamUpdate::Steps(s) => {
                self.handle_steps(s, target).await;
            }
            StreamUpdate::Tools(t) => {
                self.handle_tools(t, target).await;
            }
        }
    }

    /// Flushes remaining buffer into content and sends the final edit.
    async fn finish(&mut self, target: &EditTarget<'_>) {
        if !self.buffer.is_empty() {
            self.content.push_str(&self.buffer);
        }
        send_edit(
            target,
            &self.steps,
            &self.tools_used,
            &self.content,
            &mut self.last_edit,
        )
        .await;
    }
}

/// **Entry point.** Runs the stream-edit loop: consumes [`StreamUpdate`]s from `rx`, buffers content/steps/tools,
/// and edits the given Telegram message periodically (by chunk size or max delay).
///
/// Call via `tokio::spawn(run_stream_edit_loop(bot, chat, message_id, rx))` from
/// [`super::handler::AgentHandler::process_message`].
pub async fn run_stream_edit_loop(
    bot: BotRef,
    chat: Chat,
    message_id: String,
    mut rx: mpsc::UnboundedReceiver<StreamUpdate>,
) {
    let mut state = StreamEditState::new();
    let target = EditTarget {
        bot: &bot,
        chat: &chat,
        message_id: &message_id,
    };

    loop {
        let result = recv_with_timeout(&mut rx, state.timeout_secs()).await;
        if !state.process_recv_result(result, &target).await {
            break;
        }
    }

    state.finish(&target).await;
}
