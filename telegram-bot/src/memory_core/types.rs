//! Core types for memory storage.

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
    pub user_id: Option<String>,
    pub conversation_id: Option<String>,
    pub role: MemoryRole,
    pub timestamp: DateTime<Utc>,
    pub tokens: Option<u32>,
    pub importance: Option<f32>,
}

/// A single memory entry in the conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: MemoryMetadata,
}

impl MemoryEntry {
    pub fn new(content: String, metadata: MemoryMetadata) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            embedding: None,
            metadata,
        }
    }
}
