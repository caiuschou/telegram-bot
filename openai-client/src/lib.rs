use async_openai::{types::CreateChatCompletionRequestArgs, Client};
use futures::StreamExt;
use std::sync::Arc;
use tracing;

pub use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};

/// Masks an API key/token for safe logging: shows first 7 chars + "***" + last 4 chars.
/// If length <= 11, returns "***" to avoid leaking any part of the key.
/// Exposed for tests and for callers who need to log API keys safely.
pub fn mask_token(token: &str) -> String {
    let len = token.len();
    if len <= 11 {
        "***".to_string()
    } else {
        let head_len = 7.min(len);
        let tail_len = 4.min(len.saturating_sub(head_len));
        let head = &token[..head_len];
        let tail = if tail_len > 0 {
            &token[len - tail_len..]
        } else {
            ""
        };
        format!("{}***{}", head, tail)
    }
}

#[derive(Clone)]
pub struct OpenAIClient {
    client: Arc<Client<async_openai::config::OpenAIConfig>>,
    /// API key stored only for logging (masked). None when created via with_client().
    api_key_for_logging: Option<String>,
}

pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        let api_key_for_logging = Some(api_key.clone());
        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self {
            client: Arc::new(client),
            api_key_for_logging,
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let api_key_for_logging = Some(api_key.clone());
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        let client = Client::with_config(config);
        Self {
            client: Arc::new(client),
            api_key_for_logging,
        }
    }

    pub fn with_client(client: Client<async_openai::config::OpenAIConfig>) -> Self {
        Self {
            client: Arc::new(client),
            api_key_for_logging: None,
        }
    }

    pub async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> anyhow::Result<String> {
        let message_count = messages.len();
        let masked = self
            .api_key_for_logging
            .as_deref()
            .map(mask_token)
            .unwrap_or_else(|| "***".to_string());

        tracing::info!(
            model = %model,
            message_count = message_count,
            api_key = %masked,
            "OpenAI chat_completion request"
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages)
            .build()?;

        if let Ok(json) = serde_json::to_string_pretty(&request) {
            tracing::info!(request_json = %json, "OpenAI chat_completion 提交的 JSON");
        }

        let response = self.client.chat().create(request).await?;

        if let Some(ref u) = response.usage {
            tracing::info!(
                prompt_tokens = u.prompt_tokens,
                completion_tokens = u.completion_tokens,
                total_tokens = u.total_tokens,
                "OpenAI chat_completion usage"
            );
        }

        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone().unwrap_or_default())
        } else {
            anyhow::bail!("No response from OpenAI");
        }
    }

    pub async fn chat_completion_stream<F, Fut>(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
        mut callback: F,
    ) -> anyhow::Result<String>
    where
        F: FnMut(StreamChunk) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        let message_count = messages.len();
        let masked = self
            .api_key_for_logging
            .as_deref()
            .map(mask_token)
            .unwrap_or_else(|| "***".to_string());

        tracing::info!(
            model = %model,
            message_count = message_count,
            api_key = %masked,
            "OpenAI chat_completion_stream request"
        );

        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages)
            .build()?;

        if let Ok(json) = serde_json::to_string_pretty(&request) {
            tracing::info!(request_json = %json, "OpenAI chat_completion_stream 提交的 JSON");
        }

        let mut stream = self.client.chat().create_stream(request).await?;

        let mut full_response = String::new();
        let mut last_update = std::time::Instant::now();
        let mut pending_content = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    if let Some(ref u) = chunk.usage {
                        tracing::info!(
                            prompt_tokens = u.prompt_tokens,
                            completion_tokens = u.completion_tokens,
                            total_tokens = u.total_tokens,
                            "OpenAI chat_completion_stream usage"
                        );
                    }
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            pending_content.push_str(content);
                            full_response.push_str(content);
                        }

                        if last_update.elapsed() >= std::time::Duration::from_secs(1)
                            || choice.finish_reason.is_some()
                        {
                            if !pending_content.is_empty() {
                                callback(StreamChunk {
                                    content: pending_content.clone(),
                                    done: choice.finish_reason.is_some(),
                                })
                                .await?;
                                pending_content.clear();
                            }
                            last_update = std::time::Instant::now();
                        }
                    }
                }
                Err(e) => {
                    anyhow::bail!("Stream error: {}", e);
                }
            }
        }

        if !pending_content.is_empty() {
            callback(StreamChunk {
                content: pending_content,
                done: true,
            })
            .await?;
        }

        Ok(full_response)
    }
}
