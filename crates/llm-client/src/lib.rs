//! # LLM client abstraction
//!
//! Defines the [`LlmClient`] trait and an OpenAI implementation. Transport-agnostic;
//! used by llm-handlers and telegram-bot-llm.
//!
//! The stream method uses a boxed callback so that [`LlmClient`] is object-safe (dyn compatible).

use anyhow::Result;
use async_trait::async_trait;
use openai_client::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
};
use prompt::{ChatMessage, MessageRole};
use std::future::Future;
use std::pin::Pin;

mod openai_llm;

pub use openai_llm::OpenAILlmClient;

/// A chunk of streamed LLM output; aligned with `openai_client::StreamChunk`.
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

/// Type-erased callback for stream chunks so that [`LlmClient`] is dyn compatible.
pub type StreamChunkCallback =
    dyn FnMut(StreamChunk) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send;

/// LLM client interface: request completion or streamed completion from a list of messages.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Returns the model reply text for the given messages (system/user/assistant). Implementations add system prompt etc.
    async fn get_llm_response_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String>;

    /// Streamed completion: invokes `callback` for each chunk and returns the full reply text.
    /// Uses boxed callback for object safety (dyn LlmClient).
    async fn get_llm_response_stream_with_messages(
        &self,
        messages: Vec<ChatMessage>,
        callback: &mut StreamChunkCallback,
    ) -> Result<String>;
}

/// Converts a single [`ChatMessage`] into OpenAI API message format.
fn chat_message_to_openai(msg: &ChatMessage) -> Result<ChatCompletionRequestMessage> {
    use openai_client::ChatCompletionRequestAssistantMessageArgs;
    let content = msg.content.clone();
    let openai_msg: ChatCompletionRequestMessage = match msg.role {
        MessageRole::System => ChatCompletionRequestSystemMessageArgs::default()
            .content(content)
            .build()?
            .into(),
        MessageRole::User => ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?
            .into(),
        MessageRole::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
            .content(content)
            .build()?
            .into(),
    };
    Ok(openai_msg)
}
