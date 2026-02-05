//! Embedding configuration: trait and env-based implementation.

use anyhow::Result;
use std::env;

/// Embedding service configuration interface.
pub trait EmbeddingConfig: Send + Sync {
    fn provider(&self) -> &str;
    fn bigmodel_api_key(&self) -> &str;
    /// API key for OpenAI-compatible embedding (OPENAI_API_KEY). Used when provider is openai.
    fn openai_api_key(&self) -> &str;
    /// Optional base URL for OpenAI-compatible embedding (OPENAI_BASE_URL). When set, e.g. for Big Model, embedding requests use this URL.
    fn openai_base_url(&self) -> Option<&str>;
}

/// Embedding config loaded from environment variables.
#[derive(Debug, Clone)]
pub struct EnvEmbeddingConfig {
    pub embedding_provider: String,
    pub bigmodel_api_key: String,
    pub openai_api_key: String,
    pub openai_base_url: Option<String>,
}

impl EmbeddingConfig for EnvEmbeddingConfig {
    fn provider(&self) -> &str {
        &self.embedding_provider
    }
    fn bigmodel_api_key(&self) -> &str {
        &self.bigmodel_api_key
    }
    fn openai_api_key(&self) -> &str {
        &self.openai_api_key
    }
    fn openai_base_url(&self) -> Option<&str> {
        self.openai_base_url.as_deref().filter(|s| !s.is_empty())
    }
}

impl EnvEmbeddingConfig {
    /// Load from environment variables.
    pub fn from_env() -> Result<Self> {
        let embedding_provider =
            env::var("EMBEDDING_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let bigmodel_api_key = env::var("BIGMODEL_API_KEY")
            .or_else(|_| env::var("ZHIPUAI_API_KEY"))
            .unwrap_or_default();
        let openai_api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        let openai_base_url = env::var("OPENAI_BASE_URL").ok().filter(|s| !s.trim().is_empty());
        Ok(Self {
            embedding_provider,
            bigmodel_api_key,
            openai_api_key,
            openai_base_url,
        })
    }

    /// Validate config (e.g. zhipuai requires BIGMODEL_API_KEY).
    pub fn validate(&self) -> Result<()> {
        if self.embedding_provider.eq_ignore_ascii_case("zhipuai")
            && self.bigmodel_api_key.is_empty()
        {
            anyhow::bail!(
                "EMBEDDING_PROVIDER=zhipuai requires BIGMODEL_API_KEY or ZHIPUAI_API_KEY to be set"
            );
        }
        Ok(())
    }
}
