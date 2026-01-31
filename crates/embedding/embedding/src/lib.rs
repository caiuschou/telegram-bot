//! # Text Embeddings
//!
//! This crate defines the embedding service interface for generating text embeddings.
//!
//! ## EmbeddingService Trait
//!
//! The `EmbeddingService` trait defines the interface for generating text embeddings.
//!
//! ### Required Methods
//!
//! #### `embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error>`
//!
//! Generates an embedding vector for a single text string.
//!
//! #### `embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error>`
//!
//! Generates embedding vectors for multiple texts in a single API call. This is more efficient than calling `embed` multiple times.
//!
//! ## Example Usage
//!
//! ```rust
//! use embedding::EmbeddingService;
//!
//! async fn example(service: &impl EmbeddingService) -> Result<(), anyhow::Error> {
//!     // Single text embedding
//!     let embedding = service.embed("Hello world").await?;
//!     println!("Embedding dimension: {}", embedding.len());
//!
//!     // Batch embedding
//!     let texts = vec![
//!         "Hello".to_string(),
//!         "World".to_string(),
//!         "Goodbye".to_string(),
//!     ];
//!     let embeddings = service.embed_batch(&texts).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Embedding Best Practices
//!
//! ### Batch Processing
//!
//! Always use `embed_batch` when processing multiple texts to reduce API calls and costs.
//!
//! ### Dimensionality
//!
//! Choose an embedding dimension based on your use case:
//! - **Small (384-768)**: Faster, lower cost, good for most use cases
//! - **Medium (1024-1536)**: Better semantic understanding
//! - **Large (3072+)**: Maximum accuracy, higher cost
//!
//! ### Normalization
//!
//! Embeddings should be normalized for cosine similarity calculations.
//!
//! ### Caching
//!
//! Consider caching embeddings for frequently used texts to reduce API calls.

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
