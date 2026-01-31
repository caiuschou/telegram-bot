//! Text embeddings: trait and implementations (OpenAI, BigModel).

use async_trait::async_trait;

mod config;
pub mod openai;
pub mod bigmodel;

pub use config::{EmbeddingConfig, EnvEmbeddingConfig};
pub use openai::OpenAIEmbedding;
pub use bigmodel::BigModelEmbedding;

/// Service for generating text embeddings.
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error>;
}
