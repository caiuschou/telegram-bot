//! App extensions trait and default implementation (memory, embedding, optional LLM system prompt).
//! LLM config (model, API, etc.) is implemented externally in llm-client.

use anyhow::Result;
use std::env;
use crate::embedding::{EmbeddingConfig, EnvEmbeddingConfig};
use crate::memory::{EnvMemoryConfig, MemoryConfig};

/// Application extension config. Implement this trait to inject custom config.
pub trait AppExtensions: Send + Sync {
    fn memory_config(&self) -> Option<&dyn MemoryConfig>;
    fn embedding_config(&self) -> Option<&dyn EmbeddingConfig>;
    /// LLM system prompt (LLM_SYSTEM_PROMPT or SYSTEM_PROMPT). Default impl returns None.
    fn llm_system_prompt(&self) -> Option<&str> {
        None
    }
}

/// Base extensions: memory + embedding + optional LLM system prompt. Used by telegram-bot framework.
pub struct BaseAppExtensions {
    pub memory: EnvMemoryConfig,
    pub embedding: EnvEmbeddingConfig,
    pub llm_system_prompt: Option<String>,
}

impl AppExtensions for BaseAppExtensions {
    fn memory_config(&self) -> Option<&dyn MemoryConfig> {
        Some(&self.memory)
    }
    fn embedding_config(&self) -> Option<&dyn EmbeddingConfig> {
        Some(&self.embedding)
    }
    fn llm_system_prompt(&self) -> Option<&str> {
        self.llm_system_prompt.as_deref()
    }
}

impl BaseAppExtensions {
    /// Load from environment variables (memory + embedding + optional LLM system prompt).
    pub fn from_env() -> Result<Self> {
        let memory = EnvMemoryConfig::from_env()?;
        let embedding = EnvEmbeddingConfig::from_env()?;
        embedding.validate()?;
        let llm_system_prompt = env::var("LLM_SYSTEM_PROMPT")
            .or_else(|_| env::var("SYSTEM_PROMPT"))
            .ok()
            .filter(|s| !s.trim().is_empty());
        Ok(Self {
            memory,
            embedding,
            llm_system_prompt,
        })
    }
}
