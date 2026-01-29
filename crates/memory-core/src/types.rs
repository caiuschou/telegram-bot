//! # Core Types
//!
//! This module defines the core types for memory storage.
//!
//! ## MemoryRole
//!
//! Represents the role of a message in a conversation.
//!
//! ### Variants
//!
//! - `User`: Message sent by the user
//! - `Assistant`: Message sent by the AI assistant
//! - `System`: System-level message
//!
//! ## MemoryMetadata
//!
//! Metadata associated with a memory entry.
//!
//! ## MemoryEntry
//!
//! A single memory entry in the conversation history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents the role of a message in a conversation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MemoryRole {
    User,
    Assistant,
    System,
}

/// Metadata associated with a memory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryMetadata {
    /// Unique identifier of the user
    pub user_id: Option<String>,
    /// Unique identifier of the conversation
    pub conversation_id: Option<String>,
    /// Role of the message sender
    pub role: MemoryRole,
    /// When the message was created
    pub timestamp: DateTime<Utc>,
    /// Estimated token count
    pub tokens: Option<u32>,
    /// Importance score (0.0 to 1.0)
    pub importance: Option<f32>,
}

/// A single memory entry in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier
    pub id: Uuid,
    /// The actual message content
    pub content: String,
    /// Vector embedding for semantic search
    pub embedding: Option<Vec<f32>>,
    /// Associated metadata
    pub metadata: MemoryMetadata,
}

impl MemoryEntry {
    /// Creates a new `MemoryEntry` with a generated UUID and no embedding.
    pub fn new(content: String, metadata: MemoryMetadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            embedding: None,
            metadata,
        }
    }
}
