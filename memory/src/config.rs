//! Memory configuration: trait and env-based implementation.

use anyhow::Result;
use std::env;

/// Memory storage and RAG strategy configuration interface.
pub trait MemoryConfig: Send + Sync {
    fn store_type(&self) -> &str;
    fn sqlite_path(&self) -> &str;
    fn recent_use_sqlite(&self) -> bool;
    fn lance_path(&self) -> Option<&str>;
    fn recent_limit(&self) -> u32;
    fn relevant_top_k(&self) -> u32;
    fn semantic_min_score(&self) -> f32;
}

/// Memory config loaded from environment variables.
#[derive(Debug, Clone)]
pub struct EnvMemoryConfig {
    pub memory_store_type: String,
    pub memory_sqlite_path: String,
    pub memory_recent_use_sqlite: bool,
    pub memory_lance_path: Option<String>,
    pub memory_recent_limit: u32,
    pub memory_relevant_top_k: u32,
    pub memory_semantic_min_score: f32,
}

impl MemoryConfig for EnvMemoryConfig {
    fn store_type(&self) -> &str {
        &self.memory_store_type
    }
    fn sqlite_path(&self) -> &str {
        &self.memory_sqlite_path
    }
    fn recent_use_sqlite(&self) -> bool {
        self.memory_recent_use_sqlite
    }
    fn lance_path(&self) -> Option<&str> {
        self.memory_lance_path.as_deref()
    }
    fn recent_limit(&self) -> u32 {
        self.memory_recent_limit
    }
    fn relevant_top_k(&self) -> u32 {
        self.memory_relevant_top_k
    }
    fn semantic_min_score(&self) -> f32 {
        self.memory_semantic_min_score
    }
}

impl EnvMemoryConfig {
    /// Load from environment variables.
    pub fn from_env() -> Result<Self> {
        let memory_store_type =
            env::var("MEMORY_STORE_TYPE").unwrap_or_else(|_| "memory".to_string());
        let memory_sqlite_path =
            env::var("MEMORY_SQLITE_PATH").unwrap_or_else(|_| "./data/memory.db".to_string());
        let memory_recent_use_sqlite = env::var("MEMORY_RECENT_USE_SQLITE")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "1" | "true" | "yes" => Some(true),
                _ => s.parse().ok(),
            })
            .unwrap_or(false);
        let memory_lance_path = env::var("MEMORY_LANCE_PATH")
            .or_else(|_| env::var("LANCE_DB_PATH"))
            .ok();
        let memory_recent_limit = env::var("MEMORY_RECENT_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        let memory_relevant_top_k = env::var("MEMORY_RELEVANT_TOP_K")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let memory_semantic_min_score = env::var("MEMORY_SEMANTIC_MIN_SCORE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        Ok(Self {
            memory_store_type,
            memory_sqlite_path,
            memory_recent_use_sqlite,
            memory_lance_path,
            memory_recent_limit,
            memory_relevant_top_k,
            memory_semantic_min_score,
        })
    }
}
