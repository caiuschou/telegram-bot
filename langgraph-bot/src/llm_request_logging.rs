//! Middleware that logs LLM request and response when the think node runs.
//!
//! ## Application-level (本模块)
//!
//! When `LLM_LOG_REQUEST` or `VERBOSE` env is set to a truthy value (e.g. 1, true, yes),
//! logs the **messages** sent to the LLM (request) and the **assistant text** returned (response).
//! This is the logical request/response (state.messages in, last assistant content out).
//!
//! ## HTTP/API-level（底层请求体与响应体）
//!
//! LLM 调用经 langgraph → async-openai → reqwest 发 HTTP。要打印底层 API 的请求/响应：
//!
//! - 设置 `RUST_LOG=reqwest=debug`（或 `RUST_LOG=debug`）可看到 reqwest 发出的请求（URL、部分信息）；
//! - 是否打印完整 request/response body 取决于 reqwest 的日志实现，若需完整 body 可在依赖侧加 middleware 或查 async-openai 是否支持。
//!
//! 示例：`LLM_LOG_REQUEST=1 RUST_LOG=info,reqwest=debug cargo run -p langgraph-bot -- run`

use async_trait::async_trait;
use std::pin::Pin;

use langgraph::{AgentError, Message, Next, NodeMiddleware, ReActState};

/// Middleware that logs ReActState.messages before the think node (request) and the new assistant content after (response).
///
/// Interacts with ThinkNode: state.messages is what gets passed to LlmClient::invoke.
/// Logs via tracing::info when node_id == "think" and env is enabled.
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
        let result = inner(state).await;
        if node_id == "think" && env_enabled() {
            if let Ok((ref new_state, _)) = result {
                let last_assistant = new_state
                    .messages
                    .iter()
                    .rev()
                    .find_map(|m| match m {
                        Message::Assistant(s) => Some(s.as_str()),
                        _ => None,
                    });
                match last_assistant {
                    Some(s) => {
                        tracing::info!(
                            len = s.len(),
                            "LLM response (think node)"
                        );
                        tracing::info!("{}", s);
                    }
                    None => {
                        tracing::info!("LLM response (think node): no assistant message in state");
                    }
                }
            }
        }
        result
    }
}
