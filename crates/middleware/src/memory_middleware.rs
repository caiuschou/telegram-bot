//! # Memory Middleware
//!
//! This module provides middleware for managing conversation memory in the bot runtime.
//!
//! ## MemoryMiddleware
//!
//! Middleware that automatically saves user messages and AI responses to the memory store,
//! and retrieves relevant context for AI responses.

use async_trait::async_trait;
use dbot_core::{HandlerResponse, Message, Middleware, Result};
use memory::{
    ContextBuilder, MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore,
    RecentMessagesStrategy, UserPreferencesStrategy,
};
use memory_inmemory::InMemoryVectorStore;
use std::sync::Arc;
use tracing::{error, info, instrument};
use chrono::Utc;

/// Configuration for MemoryMiddleware.
#[derive(Clone)]
pub struct MemoryConfig {
    /// Memory store instance (used by middleware and by tests for assertions).
    pub store: Arc<dyn MemoryStore>,
    /// Maximum number of recent messages to include in context
    pub max_recent_messages: usize,
    /// Maximum context tokens
    pub max_context_tokens: usize,
    /// Whether to save user messages
    pub save_user_messages: bool,
    /// Whether to save AI responses
    pub save_ai_responses: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            store: Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>,
            max_recent_messages: 10,
            max_context_tokens: 4096,
            save_user_messages: true,
            save_ai_responses: true,
        }
    }
}

/// Middleware for managing conversation memory.
pub struct MemoryMiddleware {
    /// Config exposed as pub(crate) for unit tests in memory_middleware_test.
    pub(crate) config: MemoryConfig,
}

impl MemoryMiddleware {
    /// Creates a new MemoryMiddleware with given config.
    pub fn new(config: MemoryConfig) -> Self {
        Self { config }
    }

    /// Creates a new MemoryMiddleware with default config.
    pub fn with_store(store: Arc<dyn MemoryStore>) -> Self {
        Self::new(MemoryConfig {
            store,
            ..Default::default()
        })
    }

    /// Creates a memory entry from a bot message (user role).
    /// pub(crate) for unit tests in memory_middleware_test.
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
    /// pub(crate) for unit tests in memory_middleware_test.
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

    /// Builds conversation context for a message.
    /// pub(crate) for unit tests in memory_middleware_test.
    #[allow(dead_code)]
    pub(crate) async fn build_context(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<String>> {
        let builder = ContextBuilder::new(self.config.store.clone())
            .with_strategy(Box::new(RecentMessagesStrategy::new(
                self.config.max_recent_messages,
            )))
            .with_strategy(Box::new(UserPreferencesStrategy::new()))
            .with_token_limit(self.config.max_context_tokens)
            .for_user(user_id)
            .for_conversation(conversation_id);

        let context = builder.build().await
            .map_err(|e| dbot_core::DbotError::Unknown(e.to_string()))?;

        if context.conversation_history.is_empty() {
            Ok(None)
        } else {
            let formatted = context.format_for_model(false);
            Ok(Some(formatted))
        }
    }
}

#[async_trait]
impl Middleware for MemoryMiddleware {
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool> {
        let user_id = message.user.id.to_string();
        let conversation_id = message.chat.id.to_string();

        info!(
            user_id = %user_id,
            conversation_id = %conversation_id,
            message_id = %message.id,
            "step: MemoryMiddleware before, saving user message to memory"
        );

        // Save user message to memory
        if self.config.save_user_messages {
            let entry = self.message_to_memory_entry(message);

            if let Err(e) = self.config.store.add(entry.clone()).await {
                error!(error = %e, "Failed to save user message to memory");
            } else {
                info!(
                    user_id = %user_id,
                    conversation_id = %conversation_id,
                    entry_id = %entry.id,
                    "step: MemoryMiddleware before done, user message saved to memory"
                );
            }
        } else {
            info!(
                user_id = %user_id,
                "step: MemoryMiddleware before done (save_user_messages=false, skip)"
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
            "step: MemoryMiddleware after"
        );

        // Save AI response to memory when handler returns Reply(text) and config allows.
        if self.config.save_ai_responses {
            if let HandlerResponse::Reply(text) = response {
                let entry = self.reply_to_memory_entry(message, text);
                if let Err(e) = self.config.store.add(entry.clone()).await {
                    error!(error = %e, "Failed to save AI response to memory");
                } else {
                    info!(
                        user_id = %user_id,
                        conversation_id = %conversation_id,
                        entry_id = %entry.id,
                        "step: MemoryMiddleware after done, AI reply saved to memory"
                    );
                }
            } else {
                info!(
                    user_id = %user_id,
                    "step: MemoryMiddleware after done (no Reply, skip save)"
                );
            }
        } else {
            info!(
                user_id = %user_id,
                "step: MemoryMiddleware after done (save_ai_responses=false, skip)"
            );
        }

        Ok(())
    }
}
