//! MessageRecord → MemoryEntry conversion.
//!
//! Converts SQLite message records into [`memory::MemoryEntry`] for the vector store.
//! Dependencies: [`storage::MessageRecord`], [`memory::MemoryEntry`].

use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
use storage::MessageRecord;
use uuid::Uuid;

/// Converts a [`MessageRecord`] into a [`MemoryEntry`].
///
/// # Field mapping
///
/// - `id`: Parsed from `msg.id` (UUID); on parse failure a new v4 UUID is generated.
/// - `content`: `msg.content`
/// - `embedding`: `None` (filled later by the embedding service during load).
/// - `metadata.user_id`: `msg.user_id` as string
/// - `metadata.conversation_id`: `msg.chat_id` as string
/// - `metadata.role`: From `msg.direction` — `"received"` → User, `"sent"` → Assistant, else User
/// - `metadata.timestamp`: `msg.created_at`
/// - `metadata.tokens`: `None`
/// - `metadata.importance`: `None`
///
/// # Arguments
///
/// * `msg` - SQLite message record to convert
///
/// # Returns
///
/// The converted [`MemoryEntry`] (no embedding).
pub(crate) fn convert(msg: &MessageRecord) -> MemoryEntry {
    let role = match msg.direction.as_str() {
        "received" => MemoryRole::User,
        "sent" => MemoryRole::Assistant,
        _ => MemoryRole::User,
    };

    MemoryEntry {
        id: Uuid::parse_str(&msg.id).unwrap_or_else(|_| Uuid::new_v4()),
        content: msg.content.clone(),
        embedding: None,
        metadata: MemoryMetadata {
            user_id: Some(msg.user_id.to_string()),
            conversation_id: Some(msg.chat_id.to_string()),
            role,
            timestamp: msg.created_at,
            tokens: None,
            importance: None,
        },
    }
}
