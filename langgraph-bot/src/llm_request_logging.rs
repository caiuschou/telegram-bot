//! Middleware that logs LLM request messages when the think node runs.
//!
//! When `LLM_LOG_REQUEST` or `VERBOSE` env is set to a truthy value, logs each message
//! (System/User/Assistant) sent to the LLM before invocation. Used for debugging.

use async_trait::async_trait;
use std::pin::Pin;

use langgraph::{AgentError, Message, Next, NodeMiddleware, ReActState};

/// Middleware that logs ReActState.messages before the think node runs.
///
/// Interacts with ThinkNode: state.messages is what gets passed to LlmClient::invoke.
/// Logs via tracing::info when node_id == "think".
pub struct LlmRequestLoggingMiddleware;

fn env_enabled() -> bool {
    std::env::var("LLM_LOG_REQUEST")
        .or_else(|_| std::env::var("VERBOSE"))
        .ok()
        .and_then(|s| match s.to_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            _ => s.parse().ok(),
        })
        .unwrap_or(false)
}

fn format_message(idx: usize, m: &Message) -> String {
    let (role, content) = match m {
        Message::System(s) => ("system", s.as_str()),
        Message::User(s) => ("user", s.as_str()),
        Message::Assistant(s) => ("assistant", s.as_str()),
    };
    format!("  [{}] {}: {}", idx, role, content)
}

#[async_trait]
impl NodeMiddleware<ReActState> for LlmRequestLoggingMiddleware {
    async fn around_run(
        &self,
        node_id: &str,
        state: ReActState,
        inner: Box<
            dyn FnOnce(ReActState)
                -> Pin<
                    Box<dyn std::future::Future<Output = Result<(ReActState, Next), AgentError>> + Send>,
                > + Send,
        >,
    ) -> Result<(ReActState, Next), AgentError> {
        if node_id == "think" && env_enabled() {
            tracing::info!(
                messages_count = state.messages.len(),
                turn_count = state.turn_count,
                "LLM request (think node)"
            );
            for (i, m) in state.messages.iter().enumerate() {
                tracing::info!("{}", format_message(i, m));
            }
        }
        inner(state).await
    }
}
