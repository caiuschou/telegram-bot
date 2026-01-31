//! # Text Embeddings
//!
//! This crate defines the embedding service interface for generating text embeddings.

use async_trait::async_trait;

mod config;
pub use config::{EmbeddingConfig, EnvEmbeddingConfig};

/// Service for generating text embeddings.
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    /// Generates an embedding vector for a single text string.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error>;

    /// Generates embedding vectors for multiple texts in a single API call.
    /// This is more efficient than calling `embed` multiple times.
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error>;
}
