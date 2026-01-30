//! # BigModel Embedding Service
//!
//! This crate provides an implementation of the `EmbeddingService` trait using BigModel (Zhipu AI)'s embedding API.
//!
//! ## BigModelEmbedding
//!
//! Uses BigModel's embedding models (e.g., `embedding-2`, `embedding-3`).
//!
//! **Advantages**:
//! - Chinese optimized embeddings
//! - Competitive pricing
//! - High quality semantic understanding
//!
//! **Considerations**:
//! - Requires API key
//! - Rate limits
//! - Cost per request
//!
//! ## Example
//!
//! ```rust,no_run
//! use bigmodel_embedding::BigModelEmbedding;
//! use embedding::EmbeddingService;
//!
//! fn create_service() -> BigModelEmbedding {
//!     // The API key can be provided directly or set via BIGMODEL_API_KEY environment variable
//!     BigModelEmbedding::new("your-api-key".to_string(), "embedding-2".to_string())
//! }
//!
//! async fn example(service: &BigModelEmbedding) -> Result<(), anyhow::Error> {
//!     // Generate embedding for a single text
//!     let embedding = service.embed("Hello world").await?;
//!     println!("Embedding dimension: {}", embedding.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! The `BigModelEmbedding` service requires:
//! - **API Key**: Your BigModel API key (can also be set via BIGMODEL_API_KEY environment variable)
//! - **Model**: The embedding model to use (default: `embedding-2`)
//!
//! ## Supported Models
//!
//! - `embedding-2`: 1024 dimensions, Chinese optimized
//! - `embedding-3`: 256–2048 dimensions (configurable), improved quality
//!
//! See [BigModel API Documentation](https://open.bigmodel.cn/dev/api#embeddings) for more details.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use embedding::EmbeddingService;
use tracing::info;

const BIGMODEL_API_BASE: &str = "https://open.bigmodel.cn/api/paas/v4/embeddings";

/// BigModel embedding service implementation.
#[derive(Debug, Clone)]
pub struct BigModelEmbedding {
    client: Client,
    api_key: String,
    model: String,
}

impl BigModelEmbedding {
    /// Creates a new BigModel embedding service.
    ///
    /// # Arguments
    ///
    /// * `api_key` - BigModel API key. If empty, will try to read from BIGMODEL_API_KEY environment variable.
    /// * `model` - The embedding model to use (e.g., "embedding-2", "embedding-3").
    pub fn new(api_key: String, model: String) -> Self {
        let api_key = if api_key.is_empty() {
            std::env::var("BIGMODEL_API_KEY").unwrap_or_default()
        } else {
            api_key
        };

        let client = Client::builder()
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key, model }
    }

    /// Creates a new BigModel embedding service with default model.
    ///
    /// Uses `embedding-2` as the default model.
    pub fn with_api_key(api_key: String) -> Self {
        Self::new(api_key, "embedding-2".to_string())
    }

    /// Sets a different embedding model.
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Returns the current model name.
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
    /// Generates an embedding vector for a single text string using BigModel's API.
    ///
    /// This method sends a request to BigModel's embeddings API and returns the vector representation
    /// of the input text. The embedding captures the semantic meaning of the text in a high-dimensional
    /// vector space, enabling semantic similarity comparisons.
    ///
    /// # API Interaction
    ///
    /// 1. Constructs JSON request with configured model and input text
    /// 2. Sends HTTP POST request to BigModel's embeddings endpoint
    /// 3. Parses JSON response to extract embedding vector
    /// 4. Returns the first (and only) embedding from the response
    ///
    /// # External Interactions
    ///
    /// - **BigModel API**: Makes HTTPS request to https://open.bigmodel.cn/api/paas/v4/embeddings
    /// - **Network**: Requires internet connectivity to reach BigModel servers
    /// - **Rate Limits**: Subject to BigModel's API rate limits
    /// - **Billing**: Each request consumes quota
    ///
    /// # Arguments
    ///
    /// * `text` - The text string to embed.
    ///
    /// # Returns
    ///
    /// A vector of floats representing the embedding, or an error if the API request fails.
    /// Vector dimensions depend on the configured model (e.g., 1024 for embedding-v2).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API key is not set or invalid
    /// - The API request fails (network error, timeout, rate limit, etc.)
    /// - The response is malformed or missing embeddings
    /// - Insufficient API quota
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error> {
        const LOG_PREVIEW_LEN: usize = 200;
        let text_preview = if text.len() <= LOG_PREVIEW_LEN {
            text.to_string()
        } else {
            format!("{}...", &text[..LOG_PREVIEW_LEN])
        };
        info!(
            model = %self.model,
            text_preview = %text_preview,
            text_len = text.len(),
            "step: 词向量 BigModel embed 请求"
        );

        let request = EmbeddingRequest {
            model: &self.model,
            input: Input::Single(text),
        };

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

        let embedding = embedding_response
            .data
            .first()
            .ok_or_else(|| anyhow::anyhow!("No embedding in response"))?
            .embedding
            .clone();

        info!(
            dimension = embedding.len(),
            "step: 词向量 BigModel embed 完成"
        );
        Ok(embedding)
    }

    /// Generates embedding vectors for multiple texts in a single API call.
    ///
    /// This method is more efficient than calling `embed` multiple times, as it processes
    /// all texts in a single API request. Batch processing reduces overhead and can be
    /// significantly faster and cheaper for multiple texts.
    ///
    /// # Performance Considerations
    ///
    /// - Batch size limit: Up to texts per request (BigModel API limit)
    /// - Recommended batch size: 10-100 texts for optimal performance
    /// - Network efficiency: Single request reduces latency overhead
    ///
    /// # External Interactions
    ///
    /// - **BigModel API**: Makes HTTPS request to https://open.bigmodel.cn/api/paas/v4/embeddings
    /// - **Network**: Requires internet connectivity to reach BigModel servers
    ///
    /// # Arguments
    ///
    /// * `texts` - A slice of text strings to embed.
    ///
    /// # Returns
    ///
    /// A vector of embedding vectors, one for each input text, in the same order.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input slice is empty (no-op, returns empty result)
    /// - The API key is not set or invalid
    /// - The API request fails (network error, timeout, rate limit, etc.)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        info!(
            model = %self.model,
            batch_size = texts.len(),
            "step: 词向量 BigModel embed_batch 请求"
        );

        let inputs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        let request = EmbeddingRequest {
            model: &self.model,
            input: Input::Batch(&inputs),
        };

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

        // Sort by index to ensure order matches input
        let mut data = embedding_response.data;
        data.sort_by_key(|d| d.index);

        let embeddings: Vec<Vec<f32>> = data
            .into_iter()
            .map(|item| item.embedding)
            .collect();

        if embeddings.len() != texts.len() {
            return Err(anyhow::anyhow!(
                "Expected {} embeddings, got {}",
                texts.len(),
                embeddings.len()
            ));
        }

        let dimension = embeddings.first().map(|v| v.len()).unwrap_or(0);
        info!(
            count = embeddings.len(),
            dimension = dimension,
            "step: 词向量 BigModel embed_batch 完成"
        );
        Ok(embeddings)
    }
}

#[cfg(test)]
mod bigmodel_embedding_test;
