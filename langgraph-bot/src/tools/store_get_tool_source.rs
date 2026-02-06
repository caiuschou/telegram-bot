//! Composite [`ToolSource`] that adds the `store_get` tool (semantic search) to an upstream tool source.
//!
//! When [`StoreGetToolSource`] is built with a store and embedding service, it lists
//! `store_get` in addition to upstream tools and executes it by embedding the query
//! and calling [`MemoryStore::semantic_search`]. The store should be chat-scoped
//! (e.g. from [`telegram_bot::memory::get_store`]) so retrieval is limited to the current chat.

use std::sync::Arc;

use async_trait::async_trait;
use langgraph::tool_source::{ToolCallContent, ToolSource, ToolSourceError, ToolSpec};
use serde_json::json;
use telegram_bot::embedding::EmbeddingService;
use telegram_bot::memory::MemoryStore;

/// Tool name for semantic search over the injected store.
pub const STORE_GET_TOOL_NAME: &str = "store_get";

const DEFAULT_LIMIT: usize = 5;

/// Wraps an upstream [`ToolSource`] and adds the `store_get` tool when store and embedding are provided.
pub struct StoreGetToolSource {
    upstream: Box<dyn ToolSource>,
    store: Option<Arc<dyn MemoryStore>>,
    embedding: Option<Arc<dyn EmbeddingService>>,
}

impl StoreGetToolSource {
    /// Creates a composite tool source. If both `store` and `embedding` are `Some`,
    /// the `store_get` tool is listed and can be called; otherwise only upstream tools are used.
    pub fn new(
        upstream: Box<dyn ToolSource>,
        store: Option<Arc<dyn MemoryStore>>,
        embedding: Option<Arc<dyn EmbeddingService>>,
    ) -> Self {
        Self {
            upstream,
            store,
            embedding,
        }
    }

    fn store_get_spec() -> ToolSpec {
        ToolSpec {
            name: STORE_GET_TOOL_NAME.to_string(),
            description: Some(
                "Search long-term memory by semantic similarity. Use a short natural language query."
                    .to_string(),
            ),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language search query" },
                    "limit": { "type": "integer", "description": "Max number of results (default 5)", "default": 5 }
                },
                "required": ["query"]
            }),
        }
    }
}

#[async_trait]
impl ToolSource for StoreGetToolSource {
    async fn list_tools(&self) -> Result<Vec<ToolSpec>, ToolSourceError> {
        let mut tools = self.upstream.list_tools().await?;
        if self.store.is_some() && self.embedding.is_some() {
            tools.push(Self::store_get_spec());
        }
        Ok(tools)
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolCallContent, ToolSourceError> {
        if name == STORE_GET_TOOL_NAME {
            let store = self
                .store
                .as_ref()
                .ok_or_else(|| ToolSourceError::InvalidInput("store_get: store not configured".into()))?;
            let embedding = self
                .embedding
                .as_ref()
                .ok_or_else(|| ToolSourceError::InvalidInput("store_get: embedding not configured".into()))?;

            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolSourceError::InvalidInput("store_get: missing or invalid 'query'".into()))?;
            let limit = arguments
                .get("limit")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or(DEFAULT_LIMIT)
                .min(20);

            let query_embedding = embedding
                .embed(query)
                .await
                .map_err(|e| ToolSourceError::InvalidInput(format!("store_get: embedding failed: {}", e)))?;

            let results = store
                .semantic_search(&query_embedding, limit, None, None)
                .await
                .map_err(|e| ToolSourceError::InvalidInput(format!("store_get: search failed: {}", e)))?;

            let lines: Vec<String> = results
                .into_iter()
                .enumerate()
                .map(|(i, (score, entry))| {
                    format!(
                        "{}. [score={:.3}] {}",
                        i + 1,
                        score,
                        entry.content.trim()
                    )
                })
                .collect();
            let text = if lines.is_empty() {
                "No relevant memories found.".to_string()
            } else {
                lines.join("\n")
            };
            return Ok(ToolCallContent { text });
        }

        self.upstream.call_tool(name, arguments).await
    }
}
