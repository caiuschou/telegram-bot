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
    RecentMessagesStrategy, UserPreferencesStrategy, InMemoryVectorStore,
};
use std::sync::Arc;
use tracing::{debug, error, info, instrument};
use chrono::Utc;

/// Configuration for MemoryMiddleware.
#[derive(Clone)]
pub struct MemoryConfig {
    /// Memory store instance
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
            store: Arc::new(memory::InMemoryVectorStore::new()) as Arc<dyn MemoryStore>,
            max_recent_messages: 10,
            max_context_tokens: 4096,
            save_user_messages: true,
            save_ai_responses: true,
        }
    }
}

/// Middleware for managing conversation memory.
pub struct MemoryMiddleware {
    config: MemoryConfig,
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

    /// Creates a memory entry from a bot message.
    fn message_to_memory_entry(&self, message: &Message) -> MemoryEntry {
        let role = MemoryRole::User;

        let user_id = Some(message.user.id.to_string());
        let conversation_id = Some(message.chat.id.to_string());

        let metadata = MemoryMetadata {
            user_id,
            conversation_id,
            role,
            timestamp: Utc::now(),
            tokens: None,
            importance: None,
        };

        MemoryEntry::new(message.content.clone(), metadata)
    }

    /// Builds conversation context for a message.
    async fn build_context(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        let builder = ContextBuilder::new(self.config.store.clone())
            .with_strategy(Box::new(RecentMessagesStrategy::new(
                self.config.max_recent_messages,
            )))
            .with_strategy(Box::new(UserPreferencesStrategy::new()))
            .with_token_limit(self.config.max_context_tokens)
            .for_user(user_id)
            .for_conversation(conversation_id);

        let context = builder.build().await?;

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

        debug!(
            user_id = %user_id,
            conversation_id = %conversation_id,
            "MemoryMiddleware: Processing message"
        );

        // Save user message to memory
        if self.config.save_user_messages {
            let entry = self.message_to_memory_entry(message);

            if let Err(e) = self.config.store.add(entry.clone()).await {
                error!(error = %e, "Failed to save user message to memory");
            } else {
                debug!(
                    user_id = %user_id,
                    conversation_id = %conversation_id,
                    message_id = %entry.id,
                    "Saved user message to memory"
                );
            }
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

        debug!(
            user_id = %user_id,
            conversation_id = %conversation_id,
            "MemoryMiddleware: Processing response"
        );

        // TODO: Save AI response to memory when the AI response mechanism is updated
        // The current HandlerResponse enum (Continue, Stop, Ignore) doesn't include response text
        // This requires modifying the AI integration to return the response text

        // if self.config.save_ai_responses {
        //     if let Some(ref text) = response.text {
        //         let metadata = MemoryMetadata {
        //             user_id: Some(user_id.clone()),
        //             conversation_id: Some(conversation_id.clone()),
        //             role: MemoryRole::Assistant,
        //             timestamp: Utc::now(),
        //             tokens: None,
        //             importance: None,
        //         };

        //         let entry = MemoryEntry::new(text.clone(), metadata);

        //         if let Err(e) = self.config.store.add(entry.clone()).await {
        //             error!(error = %e, "Failed to save AI response to memory");
        //         } else {
        //             debug!(
        //                 user_id = %user_id,
        //                 conversation_id = %conversation_id,
        //                 message_id = %entry.id,
        //                 "Saved AI response to memory"
        //             );
        //         }
        //     }
        // }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memory::InMemoryVectorStore;
    use dbot_core::{User, Chat};

    fn create_test_message(content: &str) -> Message {
        Message {
            id: "test_message_id".to_string(),
            content: content.to_string(),
            user: User {
                id: 123,
                username: Some("test_user".to_string()),
                first_name: Some("Test".to_string()),
                last_name: None,
            },
            chat: Chat {
                id: 456,
                chat_type: "private".to_string(),
            },
            message_type: "text".to_string(),
            direction: dbot_core::MessageDirection::Incoming,
            created_at: Utc::now(),
            reply_to_message_id: None,
        }
    }

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();
        assert_eq!(config.max_recent_messages, 10);
        assert_eq!(config.max_context_tokens, 4096);
        assert!(config.save_user_messages);
        assert!(config.save_ai_responses);
    }

    #[test]
    fn test_memory_middleware_creation() {
        let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>;
        let middleware = MemoryMiddleware::with_store(store);
        assert!(middleware.config.save_user_messages);
    }

    #[test]
    fn test_message_to_memory_entry() {
        let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>;
        let middleware = MemoryMiddleware::with_store(store);
        let message = create_test_message("Test message");

        let entry = middleware.message_to_memory_entry(&message);

        assert_eq!(entry.content, "Test message");
        assert_eq!(entry.metadata.role, MemoryRole::User);
        assert_eq!(entry.metadata.user_id, Some("123".to_string()));
        assert_eq!(entry.metadata.conversation_id, Some("456".to_string()));
    }

    #[tokio::test]
    async fn test_memory_middleware_saves_user_messages() {
        let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>;
        let middleware = MemoryMiddleware::with_store(store.clone());
        let message = create_test_message("Hello");

        middleware.before(&message).await.unwrap();

        let entries = store.search_by_user("123").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_memory_middleware_saves_ai_responses() {
        let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>;
        let middleware = MemoryMiddleware::with_store(store.clone());
        let message = create_test_message("Hello");

        let response = HandlerResponse {
            text: Some("Hi there!".to_string()),
            is_ai: true,
        };

        middleware.after(&message, &response).await.unwrap();

        let entries = store.search_by_user("123").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "Hi there!");
        assert_eq!(entries[0].metadata.role, MemoryRole::Assistant);
    }

    #[tokio::test]
    async fn test_memory_middleware_builds_context() {
        let store = Arc::new(InMemoryVectorStore::new()) as Arc<dyn MemoryStore>;
        let middleware = MemoryMiddleware::with_store(store.clone());

        let message = create_test_message("Hello");
        middleware.before(&message).await.unwrap();

        let context = middleware
            .build_context("123", "456")
            .await
            .unwrap();

        assert!(context.is_some());
        assert!(context.unwrap().contains("Hello"));
    }
}
