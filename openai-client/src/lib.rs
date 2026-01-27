use async_openai::{types::CreateChatCompletionRequestArgs, Client};
use futures::StreamExt;
use std::sync::Arc;

pub use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs,
};

#[derive(Clone)]
pub struct OpenAIClient {
    client: Arc<Client<async_openai::config::OpenAIConfig>>,
}

pub struct StreamChunk {
    pub content: String,
    pub done: bool,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        let config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self {
            client: Arc::new(client),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        let client = Client::with_config(config);
        Self {
            client: Arc::new(client),
        }
    }

    pub fn with_client(client: Client<async_openai::config::OpenAIConfig>) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    pub async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> anyhow::Result<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages)
            .build()?;

        let response = self.client.chat().create(request).await?;

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
        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages)
            .build()?;

        let mut stream = self.client.chat().create_stream(request).await?;

        let mut full_response = String::new();
        let mut last_update = std::time::Instant::now();
        let mut pending_content = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
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
