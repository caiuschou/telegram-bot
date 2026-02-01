//! LLM configuration: trait and env-based implementation.

use anyhow::{Context, Result};
use std::env;

/// LLM configuration interface for OpenAI-compatible APIs.
pub trait LlmConfig: Send + Sync {
    fn api_key(&self) -> &str;
    fn base_url(&self) -> &str;
    fn model(&self) -> &str;
    fn use_streaming(&self) -> bool;
    fn thinking_message(&self) -> &str;
    fn system_prompt(&self) -> Option<&str>;
}

/// LLM config loaded from environment variables.
#[derive(Debug, Clone)]
pub struct EnvLlmConfig {
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub llm_model: String,
    pub llm_use_streaming: bool,
    pub llm_thinking_message: String,
    pub llm_system_prompt: Option<String>,
}

impl LlmConfig for EnvLlmConfig {
    fn api_key(&self) -> &str {
        &self.openai_api_key
    }
    fn base_url(&self) -> &str {
        &self.openai_base_url
    }
    fn model(&self) -> &str {
        &self.llm_model
    }
    fn use_streaming(&self) -> bool {
        self.llm_use_streaming
    }
    fn thinking_message(&self) -> &str {
        &self.llm_thinking_message
    }
    fn system_prompt(&self) -> Option<&str> {
        self.llm_system_prompt.as_deref()
    }
}

impl EnvLlmConfig {
    /// Load from environment variables.
    pub fn from_env() -> Result<Self> {
        let openai_api_key =
            env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
        let openai_base_url = env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let llm_model = env::var("MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string());
        let llm_use_streaming = env::var("USE_STREAMING")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);
        let llm_thinking_message = env::var("THINKING_MESSAGE")
            .unwrap_or_else(|_| "Thinking...".to_string());
        let llm_system_prompt = env::var("LLM_SYSTEM_PROMPT")
            .or_else(|_| env::var("SYSTEM_PROMPT"))
            .ok()
            .filter(|s| !s.trim().is_empty());
        Ok(Self {
            openai_api_key,
            openai_base_url,
            llm_model,
            llm_use_streaming,
            llm_thinking_message,
            llm_system_prompt,
        })
    }
}
