//! # Memory handler
//!
//! Handler that saves user messages and LLM responses to the memory store (before/after).

use crate::core::{Handler, HandlerResponse, Message, Result};
use async_trait::async_trait;
use chrono::Utc;
use crate::embedding::EmbeddingService;
use crate::memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use crate::memory::InMemoryVectorStore;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Configuration for MemoryHandler.
#[derive(Clone)]
pub struct MemoryConfig {
    /// Memory store instance (used by handler and by tests for assertions). Primary store for semantic search and general persistence.
    pub store: Arc<dyn MemoryStore>,
    /// Optional store for recent messages only. When set, user messages and AI replies are also written here so RecentMessagesStrategy reads from it; semantic search still uses `store`.
    pub recent_store: Option<Arc<dyn MemoryStore>>,
    /// Optional embedding service: when set, user messages and LLM replies are embedded before saving so they participate in semantic search.
    pub embedding_service: Option<Arc<dyn EmbeddingService>>,
    /// Maximum number of recent messages to include in context
    pub max_recent_messages: usize,
    /// Maximum context tokens
    pub max_context_tokens: usize,
    /// Whether to save user messages
    pub save_user_messages: bool,
    /// Whether to save LLM responses
    pub save_llm_responses: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            store: Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>,
            recent_store: None,
            embedding_service: None,
            max_recent_messages: 10,
            max_context_tokens: 4096,
            save_user_messages: true,
            save_llm_responses: true,
        }
    }
}

/// Handler for managing conversation memory.
pub struct MemoryHandler {
    /// Config exposed as pub(crate) for unit tests in memory_handler_test.
    pub(crate) config: MemoryConfig,
}

impl MemoryHandler {
    /// Creates a new MemoryHandler with given config.
    pub fn new(config: MemoryConfig) -> Self {
        Self { config }
    }

    /// Creates a new MemoryHandler with default config.
    pub fn with_store(store: Arc<dyn MemoryStore>) -> Self {
        Self::new(MemoryConfig {
            store,
            ..Default::default()
        })
    }

    /// Creates a new MemoryHandler with store and embedding service so saved messages get embeddings and participate in semantic search.
    /// When `recent_store` is set, user messages and AI replies are also written there so recent-message strategies read from it.
    pub fn with_store_and_embedding(
        store: Arc<dyn MemoryStore>,
        embedding_service: Arc<dyn EmbeddingService>,
        recent_store: Option<Arc<dyn MemoryStore>>,
    ) -> Self {
        Self::new(MemoryConfig {
            store,
            recent_store,
            embedding_service: Some(embedding_service),
            ..Default::default()
        })
    }

    /// Creates a memory entry from a bot message (user role).
    /// pub(crate) for unit tests in memory_handler_test.
    pub(crate) fn message_to_memory_entry(&self, message: &Message) -> MemoryEntry {
        let user_id = Some(message.user.id.to_string());
        let conversation_id = Some(message.chat.id.to_string());

        let metadata = MemoryMetadata {
            user_id,
            conversation_id,
            role: MemoryRole::User,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };

        MemoryEntry::new(message.content.clone(), metadata)
    }

    /// Creates a memory entry for an assistant reply (e.g. from HandlerResponse::Reply(text)).
    /// pub(crate) for unit tests in memory_handler_test.
    pub(crate) fn reply_to_memory_entry(&self, message: &Message, reply_text: &str) -> MemoryEntry {
        let user_id = Some(message.user.id.to_string());
        let conversation_id = Some(message.chat.id.to_string());

        let metadata = MemoryMetadata {
            user_id,
            conversation_id,
            role: MemoryRole::Assistant,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };

        MemoryEntry::new(reply_text.to_string(), metadata)
    }
}

#[async_trait]
impl Handler for MemoryHandler {
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool> {
        let user_id = message.user.id.to_string();
        let conversation_id = message.chat.id.to_string();

        info!(
            user_id = %user_id,
            conversation_id = %conversation_id,
            message_id = %message.id,
            "step: MemoryHandler before, saving user message to memory"
        );

        // Save user message to memory
        if self.config.save_user_messages {
            let mut entry = self.message_to_memory_entry(message);
            if let Some(ref svc) = self.config.embedding_service {
                match svc.embed(&entry.content).await {
                    Ok(emb) => entry.embedding = Some(emb),
                    Err(e) => {
                        error!(error = %e, "Failed to embed user message, saving without embedding");
                    }
                }
            }

            if let Err(e) = self.config.store.add(entry.clone()).await {
                error!(error = %e, "Failed to save user message to memory");
            } else {
                info!(
                    user_id = %user_id,
                    conversation_id = %conversation_id,
                    entry_id = %entry.id,
                    "step: MemoryHandler before done, user message saved to memory"
                );
            }
            if let Some(ref r) = self.config.recent_store {
                if !std::ptr::addr_eq(r.as_ref() as *const _, self.config.store.as_ref() as *const _) {
                    if let Err(e) = r.add(entry).await {
                        error!(error = %e, "Failed to save user message to recent store");
                    }
                }
            }
        } else {
            info!(
                user_id = %user_id,
                "step: MemoryHandler before done (save_user_messages=false, skip)"
            );
        }

        Ok(true)
    }

    #[instrument(skip(self, message, response))]
    async fn after(
        &self,
        message: &Message,
        response: &HandlerResponse,
    ) -> Result<()> {
        let user_id = message.user.id.to_string();
        let conversation_id = message.chat.id.to_string();

        info!(
            user_id = %user_id,
            conversation_id = %conversation_id,
            has_reply = matches!(response, HandlerResponse::Reply(_)),
            "step: MemoryHandler after"
        );

        // Save LLM response to memory when handler returns Reply(text) and config allows.
        if self.config.save_llm_responses {
            if let HandlerResponse::Reply(text) = response {
                let mut entry = self.reply_to_memory_entry(message, text);
                if let Some(ref svc) = self.config.embedding_service {
                    match svc.embed(&entry.content).await {
                        Ok(emb) => entry.embedding = Some(emb),
                        Err(e) => {
                            error!(error = %e, "Failed to embed LLM reply, saving without embedding");
                        }
                    }
                }
                if let Err(e) = self.config.store.add(entry.clone()).await {
                    error!(error = %e, "Failed to save LLM response to memory");
                } else {
                    info!(
                        user_id = %user_id,
                        conversation_id = %conversation_id,
                        entry_id = %entry.id,
                        "step: MemoryHandler after done, LLM reply saved to memory"
                    );
                }
                if let Some(ref r) = self.config.recent_store {
                    if !std::ptr::addr_eq(r.as_ref() as *const _, self.config.store.as_ref() as *const _) {
                        if let Err(e) = r.add(entry).await {
                            error!(error = %e, "Failed to save LLM reply to recent store");
                        }
                    }
                }
            } else {
                info!(
                    user_id = %user_id,
                    "step: MemoryHandler after done (no Reply, skip save)"
                );
            }
        } else {
            info!(
                user_id = %user_id,
                "step: MemoryHandler after done (save_llm_responses=false, skip)"
            );
        }

        Ok(())
    }
}
