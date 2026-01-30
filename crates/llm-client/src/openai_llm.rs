//! OpenAI 实现的 LlmClient：包装 openai-client，前置 system 消息。

use anyhow::Result;
use async_trait::async_trait;
use openai_client::StreamChunk as OpenAIStreamChunk;
use prompt::ChatMessage;
use tracing::instrument;

use super::{chat_message_to_openai, LlmClient, StreamChunk};

/// 默认 system 提示（纯文本、适合 Telegram）。
pub const DEFAULT_SYSTEM_CONTENT: &str =
    "不要使用 Markdown 或任何格式化符号（如*、_、`、#等），只输出纯文本，适合在 Telegram 里直接发送。";

/// 基于 openai-client 的 LlmClient 实现。
#[derive(Clone)]
pub struct OpenAILlmClient {
    client: openai_client::OpenAIClient,
    model: String,
    system_prompt: Option<String>,
}

impl OpenAILlmClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: openai_client::OpenAIClient::new(api_key),
            model: "gpt-3.5-turbo".to_string(),
            system_prompt: None,
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            client: openai_client::OpenAIClient::with_base_url(api_key, base_url),
            model: "gpt-3.5-turbo".to_string(),
            system_prompt: None,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_system_prompt_opt(mut self, prompt: Option<String>) -> Self {
        self.system_prompt = prompt;
        self
    }

    fn system_content(&self) -> &str {
        self.system_prompt
            .as_deref()
            .unwrap_or(DEFAULT_SYSTEM_CONTENT)
    }
}

#[async_trait]
impl LlmClient for OpenAILlmClient {
    #[instrument(skip(self, messages))]
    async fn get_llm_response_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let mut openai_messages: Vec<openai_client::ChatCompletionRequestMessage> = vec![
            openai_client::ChatCompletionRequestSystemMessageArgs::default()
                .content(self.system_content().to_string())
                .build()?
                .into(),
        ];
        for msg in &messages {
            openai_messages.push(chat_message_to_openai(msg)?);
        }
        self.client
            .chat_completion(&self.model, openai_messages)
            .await
    }

    #[instrument(skip(self, messages, callback))]
    async fn get_llm_response_stream_with_messages<F, Fut>(
        &self,
        messages: Vec<ChatMessage>,
        mut callback: F,
    ) -> Result<String>
    where
        F: FnMut(StreamChunk) -> Fut + Send,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let mut openai_messages: Vec<openai_client::ChatCompletionRequestMessage> = vec![
            openai_client::ChatCompletionRequestSystemMessageArgs::default()
                .content(self.system_content().to_string())
                .build()?
                .into(),
        ];
        for msg in &messages {
            openai_messages.push(chat_message_to_openai(msg)?);
        }
        self.client
            .chat_completion_stream(&self.model, openai_messages, |chunk: OpenAIStreamChunk| {
                callback(StreamChunk {
                    content: chunk.content,
                    done: chunk.done,
                })
            })
            .await
            .map_err(|e| anyhow::anyhow!("Stream error: {}", e))
    }
}
