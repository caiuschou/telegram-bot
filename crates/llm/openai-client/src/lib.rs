//! # OpenAI API client
//!
//! Thin wrapper around [async-openai] for chat completion (non-stream and stream).
//! Provides token masking for safe logging and a simple request/response API.

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

/// OpenAI chat client. Wraps async-openai client; optionally holds API key for masked logging.
#[derive(Clone)]
pub struct OpenAIClient {
    /// Shared async-openai client used for all API calls.
    client: Arc<Client<async_openai::config::OpenAIConfig>>,
    /// API key stored only for logging (masked). None when created via `with_client()`.
    api_key_for_logging: Option<String>,
}

/// A chunk of streamed completion content and whether the stream is finished.
pub struct StreamChunk {
    /// Accumulated text for this chunk (since last callback).
    pub content: String,
    /// True if this is the final chunk for the response.
    pub done: bool,
}

impl OpenAIClient {
    /// Builds a client using the given API key and default API base URL.
    pub fn new(api_key: String) -> Self {
        let api_key_for_logging = Some(api_key.clone());
        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self {
            client: Arc::new(client),
            api_key_for_logging,
        }
    }

    /// Builds a client with a custom base URL (e.g. for proxies or compatible endpoints).
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

    /// Builds a client from an existing async-openai client (no API key stored for logging).
    pub fn with_client(client: Client<async_openai::config::OpenAIConfig>) -> Self {
        Self {
            client: Arc::new(client),
            api_key_for_logging: None,
        }
    }

    /// Sends a chat completion request and returns the full assistant reply as a string.
    ///
    /// Logs masked API key, request JSON, and token usage. Returns the first choice's content
    /// or an error if the response has no choices.
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
            tracing::info!(request_json = %json, "OpenAI chat_completion request JSON");
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

    /// Streams chat completion and invokes `callback` for each chunk (about every 1s or on finish).
    /// Returns the full concatenated response text. Stream errors are propagated.
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
            tracing::info!(request_json = %json, "OpenAI chat_completion_stream request JSON");
        }

        let mut stream = self.client.chat().create_stream(request).await?;

        let mut full_response = String::new();
        let mut last_update = std::time::Instant::now();
        // Accumulates content since last callback; flushed every ~1s or on finish.
        let mut pending_content = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    // Log usage when present (often on last chunk).
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
                        // Flush callback every 1 second or when the model signals finish.
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
