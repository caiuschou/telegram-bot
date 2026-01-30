//! OpenAI DALL-E 图片生成客户端
//! 
//! 提供文生图功能，调用 OpenAI DALL-E API 生成图片。

use anyhow::Result;
use async_openai::{
    config::OpenAIConfig,
    types::{CreateImageRequestArgs, ImageSize, ResponseFormat},
    Client,
};
use std::sync::Arc;
use tracing;

/// OpenAI DALL-E 图片生成客户端
#[derive(Clone)]
pub struct ImageGenerationClient {
    client: Arc<Client<OpenAIConfig>>,
    model: String,
    size: ImageSize,
    api_key_for_logging: Option<String>,
}

impl ImageGenerationClient {
    /// 创建新的图片生成客户端
    pub fn new(api_key: String) -> Self {
        let api_key_for_logging = Some(api_key.clone());
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self {
            client: Arc::new(client),
            model: "dall-e-3".to_string(),
            size: ImageSize::S1024x1024,
            api_key_for_logging,
        }
    }

    /// 使用自定义 base URL 创建客户端（用于兼容其他 OpenAI API 服务）
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let api_key_for_logging = Some(api_key.clone());
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        let client = Client::with_config(config);
        Self {
            client: Arc::new(client),
            model: "dall-e-3".to_string(),
            size: ImageSize::S1024x1024,
            api_key_for_logging,
        }
    }

    /// 设置模型（dall-e-2 或 dall-e-3）
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// 设置图片尺寸
    pub fn with_size(mut self, size: ImageSize) -> Self {
        self.size = size;
        self
    }

    /// 生成图片
    /// 
    /// # 参数
    /// - `prompt`: 图片描述文本
    /// 
    /// # 返回
    /// 返回生成的图片 URL
    #[tracing::instrument(skip(self))]
    pub async fn generate_image(&self, prompt: &str) -> Result<String> {
        let masked = self
            .api_key_for_logging
            .as_deref()
            .map(|k| {
                if k.len() <= 11 {
                    "***".to_string()
                } else {
                    format!("{}***{}", &k[..7.min(k.len())], &k[k.len().saturating_sub(4)..])
                }
            })
            .unwrap_or_else(|| "***".to_string());

        tracing::info!(
            model = %self.model,
            size = ?self.size,
            prompt_preview = %prompt.chars().take(100).collect::<String>(),
            api_key = %masked,
            "OpenAI image generation request"
        );

        let request = CreateImageRequestArgs::default()
            .prompt(prompt)
            .model(&self.model)
            .size(self.size)
            .response_format(ResponseFormat::Url)
            .n(1)
            .build()?;

        if let Ok(json) = serde_json::to_string_pretty(&request) {
            tracing::info!(request_json = %json, "OpenAI image generation 提交的 JSON");
        }

        let response = self.client.images().create(request).await?;

        if let Some(url) = response.data.first().and_then(|d| d.url.as_ref()) {
            tracing::info!(
                image_url = %url,
                "OpenAI image generation completed"
            );
            Ok(url.clone())
        } else {
            anyhow::bail!("No image URL in response");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要真实的 API key
    async fn test_generate_image() {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap();
        let client = ImageGenerationClient::new(api_key);
        let url = client
            .generate_image("a cute cat playing with a ball")
            .await
            .unwrap();
        assert!(!url.is_empty());
        println!("Generated image URL: {}", url);
    }
}
