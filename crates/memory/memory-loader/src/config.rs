//! Load and embedding configuration.
//!
//! Defines [`EmbeddingProvider`], [`EmbeddingConfig`], [`LoadConfig`], [`LoadResult`],
//! and logic to create an [`EmbeddingService`] from config. External: OpenAI or Zhipu API per provider.

use std::sync::Arc;

use bigmodel_embedding::BigModelEmbedding;
use embedding::EmbeddingService;
use openai_embedding::OpenAIEmbedding;

/// Embedding service provider. Matches .env `EMBEDDING_PROVIDER`: openai | zhipuai.
pub enum EmbeddingProvider {
    OpenAI,
    Zhipuai,
}

/// Embedding config: provider, optional model, API keys. Can be built from .env (EMBEDDING_PROVIDER, EMBEDDING_MODEL, OPENAI_API_KEY, BIGMODEL_API_KEY).
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    /// Model name; None = provider default.
    pub model: Option<String>,
    pub openai_api_key: String,
    pub bigmodel_api_key: String,
}

/// Data load config: SQLite URL, LanceDB path, embedding config, batch size. Loaded from env for load pipeline.
pub struct LoadConfig {
    pub database_url: String,
    pub lance_db_path: String,
    pub embedding: EmbeddingConfig,
    pub batch_size: usize,
}

/// Result of a load run: total messages, loaded count, elapsed seconds.
pub struct LoadResult {
    pub total: usize,
    pub loaded: usize,
    pub elapsed_secs: u64,
}

/// Returns the embedding dimension for the given config. LanceDB table schema embedding_dim must match this.
pub(crate) fn embedding_dim_for_config(config: &EmbeddingConfig) -> usize {
    match config.provider {
        EmbeddingProvider::OpenAI => config
            .model
            .as_deref()
            .map(|m| match m {
                "text-embedding-3-large" => 3072,
                "text-embedding-ada-002" => 1536,
                _ => 1536, // text-embedding-3-small 等默认
            })
            .unwrap_or(1536),
        EmbeddingProvider::Zhipuai => config
            .model
            .as_deref()
            .map(|m| if m.starts_with("embedding-3") { 2048 } else { 1024 })
            .unwrap_or(1024), // embedding-2 default 1024
    }
}

/// Creates an [`EmbeddingService`] from config (OpenAI or Zhipu per provider).
pub(crate) fn create_embedding_service(
    config: &EmbeddingConfig,
) -> Arc<dyn EmbeddingService + Send + Sync> {
    match config.provider {
        EmbeddingProvider::OpenAI => {
            let model = config
                .model
                .clone()
                .unwrap_or_else(|| "text-embedding-3-small".to_string());
            Arc::new(OpenAIEmbedding::new(config.openai_api_key.clone(), model))
        }
        EmbeddingProvider::Zhipuai => {
            let model = config
                .model
                .clone()
                .unwrap_or_else(|| "embedding-2".to_string());
            Arc::new(BigModelEmbedding::new(config.bigmodel_api_key.clone(), model))
        }
    }
}
