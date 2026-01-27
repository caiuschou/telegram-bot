//! # Core Types
//!
//! This module defines the core types for the memory crate.
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
//! ### Example
//!
//! ```rust
//! use memory::MemoryRole;
//!
//! let role = MemoryRole::User;
//! ```
//!
//! ## MemoryMetadata
//!
//! Metadata associated with a memory entry.
//!
//! ### Fields
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `user_id` | `Option<String>` | Unique identifier of the user |
//! | `conversation_id` | `Option<String>` | Unique identifier of the conversation |
//! | `role` | `MemoryRole` | Role of the message sender |
//! | `timestamp` | `DateTime<Utc>` | When the message was created |
//! | `tokens` | `Option<u32>` | Estimated token count |
//! | `importance` | `Option<f32>` | Importance score (0.0 to 1.0) |
//!
//! ### Example
//!
//! ```rust
//! use memory::{MemoryMetadata, MemoryRole};
//! use chrono::Utc;
//!
//! let metadata = MemoryMetadata {
//!     user_id: Some("user123".to_string()),
//!     conversation_id: Some("conv456".to_string()),
//!     role: MemoryRole::User,
//!     timestamp: Utc::now(),
//!     tokens: Some(10),
//!     importance: Some(0.8),
//! };
//! ```
//!
//! ## MemoryEntry
//!
//! A single memory entry in the conversation history.
//!
//! ### Fields
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `id` | `Uuid` | Unique identifier |
//! | `content` | `String` | The actual message content |
//! | `embedding` | `Option<Vec<f32>>` | Vector embedding for semantic search |
//! | `metadata` | `MemoryMetadata` | Associated metadata |
//!
//! ### Methods
//!
//! #### `new(content: String, metadata: MemoryMetadata) -> Self`
//!
//! Creates a new `MemoryEntry` with a generated UUID and no embedding.
//!
//! ### Example
//!
//! ```rust
//! use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
//! use chrono::Utc;
//!
//! let metadata = MemoryMetadata {
//!     user_id: Some("user123".to_string()),
//!     conversation_id: None,
//!     role: MemoryRole::User,
//!     timestamp: Utc::now(),
//!     tokens: None,
//!     importance: None,
//! };
//!
//! let entry = MemoryEntry::new("Hello world".to_string(), metadata);
//! ```
//!
//! ## Serialization
//!
//! All types implement `Serialize` and `Deserialize`, allowing easy JSON serialization:
//!
//! ```rust
//! use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
//! use serde_json;
//! use chrono::Utc;
//!
//! fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let metadata = MemoryMetadata {
//!         user_id: Some("user123".to_string()),
//!         conversation_id: None,
//!         role: MemoryRole::User,
//!         timestamp: Utc::now(),
//!         tokens: None,
//!         importance: None,
//!     };
//!     let entry = MemoryEntry::new("Test content".to_string(), metadata);
//!     
//!     let json = serde_json::to_string(&entry)?;
//!     let deserialized: MemoryEntry = serde_json::from_str(&json)?;
//!     Ok(())
//! }
//! ```

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
