//! # llm-client
//!
//! LLM 调用抽象：LlmClient trait + OpenAI 实现。与传输（Telegram）无关，供 llm-handlers、telegram-bot-llm 使用。

use anyhow::Result;
use async_trait::async_trait;
use openai_client::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
};
use prompt::{ChatMessage, MessageRole};

mod openai_llm;

pub use openai_llm::OpenAILlmClient;

/// 流式响应的一块内容，与 openai-client::StreamChunk 对齐。
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

/// LLM 客户端抽象：按消息列表请求完成或流式完成。
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// 给定消息列表（含 system/user/assistant），返回模型回复文本。实现方负责前置 system 消息等。
    async fn get_llm_response_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String>;

    /// 流式完成：对每个 chunk 调用 callback，返回完整回复文本。
    async fn get_llm_response_stream_with_messages<F, Fut>(
        &self,
        messages: Vec<ChatMessage>,
        callback: F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> Fut + Send,
        Fut: std::future::Future<Output = Result<()>> + Send;
}

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
