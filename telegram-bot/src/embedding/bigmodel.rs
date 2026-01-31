//! BigModel (Zhipu AI) embedding service.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::EmbeddingService;

const BIGMODEL_API_BASE: &str = "https://open.bigmodel.cn/api/paas/v4/embeddings";

#[derive(Debug, Clone)]
pub struct BigModelEmbedding {
    client: Client,
    api_key: String,
    model: String,
}

impl BigModelEmbedding {
    pub fn new(api_key: String, model: String) -> Self {
        let api_key = if api_key.is_empty() {
            std::env::var("BIGMODEL_API_KEY").unwrap_or_default()
        } else {
            api_key
        };
        let client = Client::builder().build().expect("Failed to create HTTP client");
        Self { client, api_key, model }
    }

    pub fn with_api_key(api_key: String) -> Self {
        Self::new(api_key, "embedding-2".to_string())
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: Input<'a>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Input<'a> {
    Single(&'a str),
    Batch(&'a [&'a str]),
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    #[serde(default)]
    _model: String,
    #[serde(default)]
    _usage: Usage,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize, Default)]
struct Usage {
    #[serde(default)]
    _total_tokens: u32,
}

#[async_trait]
impl EmbeddingService for BigModelEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error> {
        const LOG_PREVIEW_LEN: usize = 200;
        let text_preview = if text.len() <= LOG_PREVIEW_LEN { text.to_string() } else { format!("{}...", &text[..LOG_PREVIEW_LEN]) };
        info!(model = %self.model, text_preview = %text_preview, text_len = text.len(), "step: embedding BigModel embed request");

        let request = EmbeddingRequest { model: &self.model, input: Input::Single(text) };
        let response = self.client
            .post(BIGMODEL_API_BASE)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("BigModel API error ({}): {}", status, error_text));
        }
        let embedding_response: EmbeddingResponse = response.json().await?;
        let embedding = embedding_response.data.first().ok_or_else(|| anyhow::anyhow!("No embedding in response"))?.embedding.clone();
        info!(dimension = embedding.len(), "step: embedding BigModel embed done");
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        info!(model = %self.model, batch_size = texts.len(), "step: embedding BigModel embed_batch request");
        let inputs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let request = EmbeddingRequest { model: &self.model, input: Input::Batch(&inputs) };
        let response = self.client
            .post(BIGMODEL_API_BASE)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("BigModel API error ({}): {}", status, error_text));
        }
        let embedding_response: EmbeddingResponse = response.json().await?;
        let mut data = embedding_response.data;
        data.sort_by_key(|d| d.index);
        let embeddings: Vec<Vec<f32>> = data.into_iter().map(|item| item.embedding).collect();
        if embeddings.len() != texts.len() {
            return Err(anyhow::anyhow!("Expected {} embeddings, got {}", texts.len(), embeddings.len()));
        }
        let dimension = embeddings.first().map(|v| v.len()).unwrap_or(0);
        info!(count = embeddings.len(), dimension = dimension, "step: embedding BigModel embed_batch done");
        Ok(embeddings)
    }
}
