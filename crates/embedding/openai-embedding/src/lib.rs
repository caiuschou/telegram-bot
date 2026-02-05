//! # OpenAI Embedding Service
//!
//! This crate provides an implementation of the `EmbeddingService` trait using OpenAI's embedding API.
//!
//! ## OpenAIEmbedding
//!
//! Uses OpenAI's embedding models (e.g., `text-embedding-3-small`, `text-embedding-3-large`).
//!
//! **Advantages**:
//! - High quality embeddings
//! - Well-documented
//! - Multiple model options
//!
//! **Considerations**:
//! - Requires API key
//! - Rate limits
//! - Cost per request
//!
//! ## Example
//!
//! ```rust,no_run
//! use openai_embedding::OpenAIEmbedding;
//! use embedding::EmbeddingService;
//!
//! fn create_service() -> OpenAIEmbedding {
//!     // The API key can be provided directly or set via OPENAI_API_KEY environment variable
//!     OpenAIEmbedding::new("sk-...".to_string(), "text-embedding-3-small".to_string())
//! }
//!
//! async fn example(service: &OpenAIEmbedding) -> Result<(), anyhow::Error> {
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
//! The `OpenAIEmbedding` service requires:
//! - **API Key**: Your OpenAI API key (can also be set via OPENAI_API_KEY environment variable)
//! - **Model**: The embedding model to use (default: `text-embedding-3-small`)
//!
//! ## Supported Models
//!
//! - `text-embedding-3-small`: 1536 dimensions, cost-effective
//! - `text-embedding-3-large`: 3072 dimensions, higher accuracy
//! - `text-embedding-ada-002`: 1536 dimensions (legacy model)
//!
//! See [OpenAI Embeddings Documentation](https://platform.openai.com/docs/guides/embeddings) for more details.

use async_trait::async_trait;
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use embedding::EmbeddingService;
use tracing::{debug, info, instrument, warn};

/// OpenAI embedding service implementation. Holds the async-openai client and model name.
#[derive(Debug, Clone)]
pub struct OpenAIEmbedding {
    /// OpenAI client used for embeddings API calls.
    client: Client<async_openai::config::OpenAIConfig>,
    /// Embedding model name (e.g. "text-embedding-3-small").
    model: String,
}

impl OpenAIEmbedding {
    /// Creates a new OpenAI embedding service.
    ///
    /// # Arguments
    ///
    /// * `api_key` - OpenAI API key. If empty, will try to read from OPENAI_API_KEY environment variable.
    /// * `model` - The embedding model to use (e.g., "text-embedding-3-small", "text-embedding-3-large").
    pub fn new(api_key: String, model: String) -> Self {
        Self::new_with_base_url(api_key, model, None)
    }

    /// Creates a new OpenAI embedding service with optional base URL (e.g. for Big Model or other OpenAI-compatible endpoints).
    ///
    /// When `base_url` is `Some`, requests are sent to that URL instead of the default OpenAI API.
    pub fn new_with_base_url(
        api_key: String,
        model: String,
        base_url: Option<&str>,
    ) -> Self {
        let api_key = if api_key.is_empty() {
            std::env::var("OPENAI_API_KEY").unwrap_or_default()
        } else {
            api_key
        };

        let mut openai_config = async_openai::config::OpenAIConfig::new().with_api_key(api_key);
        if let Some(url) = base_url.filter(|s| !s.is_empty()) {
            openai_config = openai_config.with_api_base(url);
        }
        let client = Client::with_config(openai_config);

        Self { client, model }
    }

    /// Creates a new OpenAI embedding service with default model.
    ///
    /// Uses `text-embedding-3-small` as the default model.
    pub fn with_api_key(api_key: String) -> Self {
        Self::new(api_key, "text-embedding-3-small".to_string())
    }

    /// Creates a new OpenAI embedding service with default model and optional base URL (e.g. OPENAI_BASE_URL for Big Model).
    pub fn with_api_key_and_base_url(api_key: String, base_url: Option<&str>) -> Self {
        Self::new_with_base_url(
            api_key,
            "text-embedding-3-small".to_string(),
            base_url,
        )
    }

    /// Sets a different embedding model.
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Returns the embedding model name (for tests and diagnostics).
    pub fn model(&self) -> &str {
        &self.model
    }
}

#[async_trait]
impl EmbeddingService for OpenAIEmbedding {
    /// Generates an embedding vector for a single text string using OpenAI's API.
    ///
    /// This method sends a request to OpenAI's embeddings API and returns the vector representation
    /// of the input text. The embedding captures the semantic meaning of the text in a high-dimensional
    /// vector space, enabling semantic similarity comparisons.
    ///
    /// # API Interaction
    ///
    /// 1. Constructs CreateEmbeddingRequest with configured model and input text
    /// 2. Sends HTTP POST request to OpenAI's embeddings endpoint
    /// 3. Parses JSON response to extract embedding vector
    /// 4. Returns the first (and only) embedding from the response
    ///
    /// # External Interactions
    ///
    /// - **OpenAI API**: Makes HTTPS request to https://api.openai.com/v1/embeddings
    /// - **Network**: Requires internet connectivity to reach OpenAI servers
    /// - **Rate Limits**: Subject to OpenAI's API rate limits (e.g., 3000 RPM for tier 5)
    /// - **Billing**: Each request consumes quota (e.g., $0.02/1M tokens for text-embedding-3-small)
    ///
    /// # Arguments
    ///
    /// * `text` - The text string to embed.
    ///
    /// # Returns
    ///
    /// A vector of floats representing the embedding, or an error if the API request fails.
    /// Vector dimensions depend on the configured model (e.g., 1536 for text-embedding-3-small).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API key is not set or invalid
    /// - The API request fails (network error, timeout, rate limit, etc.)
    /// - The response is malformed or missing embeddings
    /// - Insufficient API quota
    #[instrument(skip(self, text), fields(model = %self.model, text_len = text.len()))]
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error> {
        // Default timeout for a single embed request (connect + request + response).
        const EMBED_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        const LOG_PREVIEW_LEN: usize = 200;
        let text_preview = if text.len() <= LOG_PREVIEW_LEN {
            text.to_string()
        } else {
            let safe_len = text.char_indices()
                .nth(LOG_PREVIEW_LEN)
                .map(|(idx, _)| idx)
                .unwrap_or(text.len());
            format!("{}...", &text[..safe_len])
        };

        info!(
            model = %self.model,
            text_preview = %text_preview,
            text_len = text.len(),
            "step: embedding OpenAI embed request"
        );

        let request = CreateEmbeddingRequestArgs::default()
            .model(self.model.clone())
            .input(vec![text])
            .build()?;

        let embeddings = self.client.embeddings();
        let create_future = embeddings.create(request);
        let response = match tokio::time::timeout(EMBED_TIMEOUT, create_future).await {
            Ok(Ok(r)) => {
                debug!("OpenAI embed response received");
                r
            }
            Ok(Err(e)) => {
                warn!(error = %e, "OpenAI embed request failed");
                return Err(e.into());
            }
            Err(_) => {
                warn!(
                    timeout_secs = EMBED_TIMEOUT.as_secs(),
                    "OpenAI embed request timed out"
                );
                return Err(anyhow::anyhow!(
                    "OpenAI embed request timed out after {} seconds",
                    EMBED_TIMEOUT.as_secs()
                ));
            }
        };

        let embedding = match response.data.first() {
            Some(item) => item.embedding.clone(),
            None => {
                warn!("OpenAI embed response has no embedding data");
                return Err(anyhow::anyhow!("No embedding in response"));
            }
        };

        info!(
            dimension = embedding.len(),
            "step: embedding OpenAI embed done"
        );
        Ok(embedding)
    }

    /// Generates embedding vectors for multiple texts in a single API call.
    ///
    /// This method is more efficient than calling `embed` multiple times, as it processes
    /// all texts in a single API request. Batch processing reduces overhead and can be
    /// significantly faster and cheaper for multiple texts.
    ///
    /// # API Interaction
    ///
    /// 1. Constructs CreateEmbeddingRequest with configured model and all input texts
    /// 2. Sends single HTTP POST request to OpenAI's embeddings endpoint
    /// 3. Parses JSON response to extract all embedding vectors
    /// 4. Validates that number of embeddings matches number of inputs
    ///
    /// # Performance Considerations
    ///
    /// - Batch size limit: Up to 2048 texts per request (OpenAI API limit)
    /// - Recommended batch size: 10-100 texts for optimal performance
    /// - Network efficiency: Single request reduces latency overhead
    /// - Cost efficiency: May be cheaper than individual requests due to batching
    ///
    /// # External Interactions
    ///
    /// - **OpenAI API**: Makes HTTPS request to https://api.openai.com/v1/embeddings
    /// - **Network**: Requires internet connectivity to reach OpenAI servers
    /// - **Rate Limits**: Single batch request consumes one rate limit token
    /// - **Billing**: Tokens are counted across all texts in the batch
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
    /// - The response is malformed or has fewer embeddings than inputs
    /// - Batch size exceeds OpenAI's limits
    #[instrument(skip(self, texts), fields(model = %self.model, batch_size = texts.len()))]
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        if texts.is_empty() {
            debug!("OpenAI embed_batch empty input, skipping");
            return Ok(vec![]);
        }

        info!(
            model = %self.model,
            batch_size = texts.len(),
            "step: embedding OpenAI embed_batch request"
        );

        // Timeout for batch request (longer than single embed due to larger payload).
        const EMBED_BATCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

        let inputs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        let request = CreateEmbeddingRequestArgs::default()
            .model(self.model.clone())
            .input(inputs)
            .build()?;

        let embeddings = self.client.embeddings();
        let create_future = embeddings.create(request);
        let response = match tokio::time::timeout(EMBED_BATCH_TIMEOUT, create_future).await {
            Ok(Ok(r)) => {
                debug!("OpenAI embed_batch response received");
                r
            }
            Ok(Err(e)) => {
                warn!(error = %e, "OpenAI embed_batch request failed");
                return Err(e.into());
            }
            Err(_) => {
                warn!(
                    timeout_secs = EMBED_BATCH_TIMEOUT.as_secs(),
                    "OpenAI embed_batch request timed out"
                );
                return Err(anyhow::anyhow!(
                    "OpenAI embed_batch request timed out after {} seconds",
                    EMBED_BATCH_TIMEOUT.as_secs()
                ));
            }
        };

        let embeddings: Vec<Vec<f32>> = response
            .data
            .into_iter()
            .map(|item| item.embedding)
            .collect();

        if embeddings.len() != texts.len() {
            warn!(
                expected = texts.len(),
                got = embeddings.len(),
                "OpenAI embed_batch response count mismatch"
            );
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
            "step: embedding OpenAI embed_batch done"
        );
        Ok(embeddings)
    }
}

// Unit/integration tests live in tests/openai_embedding_test.rs
