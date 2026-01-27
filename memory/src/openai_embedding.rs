//! # OpenAI Embedding Service
//!
//! This module provides an implementation of the `EmbeddingService` trait using OpenAI's embedding API.
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
//! ```rust
//! use memory::openai_embedding::OpenAIEmbedding;
//! use memory::EmbeddingService;
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
use crate::embedding::EmbeddingService;

/// OpenAI embedding service implementation.
#[derive(Debug, Clone)]
pub struct OpenAIEmbedding {
    client: Client<async_openai::config::OpenAIConfig>,
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
        let api_key = if api_key.is_empty() {
            std::env::var("OPENAI_API_KEY").unwrap_or_default()
        } else {
            api_key
        };

        let client = Client::with_config(
            async_openai::config::OpenAIConfig::new().with_api_key(api_key)
        );

        Self { client, model }
    }

    /// Creates a new OpenAI embedding service with default model.
    ///
    /// Uses `text-embedding-3-small` as the default model.
    pub fn with_api_key(api_key: String) -> Self {
        Self::new(api_key, "text-embedding-3-small".to_string())
    }

    /// Sets a different embedding model.
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

#[async_trait]
impl EmbeddingService for OpenAIEmbedding {
    /// Generates an embedding vector for a single text string using OpenAI's API.
    ///
    /// This method sends a request to OpenAI's embeddings API and returns the vector representation
    /// of the input text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text string to embed.
    ///
    /// # Returns
    ///
    /// A vector of floats representing the embedding, or an error if the API request fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API key is not set or invalid
    /// - The API request fails (network error, rate limit, etc.)
    /// - The response is malformed
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error> {
        let request = CreateEmbeddingRequestArgs::default()
            .model(self.model.clone())
            .input(vec![text])
            .build()?;

        let response = self.client.embeddings().create(request).await?;

        let embedding = response
            .data
            .first()
            .ok_or_else(|| anyhow::anyhow!("No embedding in response"))?
            .embedding
            .clone();

        Ok(embedding)
    }

    /// Generates embedding vectors for multiple texts in a single API call.
    ///
    /// This method is more efficient than calling `embed` multiple times, as it processes
    /// all texts in a single API request.
    ///
    /// # Arguments
    ///
    /// * `texts` - A slice of text strings to embed.
    ///
    /// # Returns
    ///
    /// A vector of embedding vectors, one for each input text.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API key is not set or invalid
    /// - The API request fails (network error, rate limit, etc.)
    /// - The response is malformed or has fewer embeddings than inputs
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let inputs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();

        let request = CreateEmbeddingRequestArgs::default()
            .model(self.model.clone())
            .input(inputs)
            .build()?;

        let response = self.client.embeddings().create(request).await?;

        let embeddings: Vec<Vec<f32>> = response
            .data
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

        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires API key, run with: cargo test -p memory -- --ignored
    async fn test_openai_embedding() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable must be set for this test");

        let service = OpenAIEmbedding::new(api_key, "text-embedding-3-small".to_string());

        let embedding = service.embed("Hello world").await.unwrap();
        assert!(!embedding.is_empty());
        assert_eq!(embedding.len(), 1536); // text-embedding-3-small produces 1536 dimensions
    }

    #[tokio::test]
    #[ignore]
    async fn test_openai_embedding_batch() {
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable must be set for this test");

        let service = OpenAIEmbedding::new(api_key, "text-embedding-3-small".to_string());

        let texts = vec![
            "Hello".to_string(),
            "World".to_string(),
            "Goodbye".to_string(),
        ];

        let embeddings = service.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert!(!embedding.is_empty());
            assert_eq!(embedding.len(), 1536);
        }
    }

    #[tokio::test]
    async fn test_openai_embedding_from_env() {
        // Should not panic even without API key (will fail on actual API call)
        let service = OpenAIEmbedding::with_api_key(String::new());
        assert_eq!(service.model, "text-embedding-3-small");
    }
}
