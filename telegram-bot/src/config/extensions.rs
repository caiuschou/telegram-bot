//! App extensions trait and default implementation (memory, embedding only).
//! LLM config is implemented externally.

use anyhow::Result;
use embedding::{EmbeddingConfig, EnvEmbeddingConfig};
use crate::memory::{EnvMemoryConfig, MemoryConfig};

/// Application extension config. Implement this trait to inject custom config.
/// LLM config is not included; implement externally.
pub trait AppExtensions: Send + Sync {
    fn memory_config(&self) -> Option<&dyn MemoryConfig>;
    fn embedding_config(&self) -> Option<&dyn EmbeddingConfig>;
}

/// Base extensions: memory + embedding only (no LLM). Used by telegram-bot framework.
pub struct BaseAppExtensions {
    pub memory: EnvMemoryConfig,
    pub embedding: EnvEmbeddingConfig,
}

impl AppExtensions for BaseAppExtensions {
    fn memory_config(&self) -> Option<&dyn MemoryConfig> {
        Some(&self.memory)
    }
    fn embedding_config(&self) -> Option<&dyn EmbeddingConfig> {
        Some(&self.embedding)
    }
}

impl BaseAppExtensions {
    /// Load from environment variables (memory + embedding only).
    pub fn from_env() -> Result<Self> {
        let memory = EnvMemoryConfig::from_env()?;
        let embedding = EnvEmbeddingConfig::from_env()?;
        embedding.validate()?;
        Ok(Self {
            memory,
            embedding,
        })
    }
}
