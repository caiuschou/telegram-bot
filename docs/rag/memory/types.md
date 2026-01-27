# Core Types

This document describes the core types defined in the `memory` crate.

## MemoryRole

Represents the role of a message in a conversation.

### Variants

- `User`: Message sent by the user
- `Assistant`: Message sent by the AI assistant
- `System`: System-level message

### Example

```rust
use memory::MemoryRole;

let role = MemoryRole::User;
```

## MemoryMetadata

Metadata associated with a memory entry.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `user_id` | `Option<String>` | Unique identifier of the user |
| `conversation_id` | `Option<String>` | Unique identifier of the conversation |
| `role` | `MemoryRole` | Role of the message sender |
| `timestamp` | `DateTime<Utc>` | When the message was created |
| `tokens` | `Option<u32>` | Estimated token count |
| `importance` | `Option<f32>` | Importance score (0.0 to 1.0) |

### Example

```rust
use memory::{MemoryMetadata, MemoryRole};
use chrono::Utc;

let metadata = MemoryMetadata {
    user_id: Some("user123".to_string()),
    conversation_id: Some("conv456".to_string()),
    role: MemoryRole::User,
    timestamp: Utc::now(),
    tokens: Some(10),
    importance: Some(0.8),
};
```

## MemoryEntry

A single memory entry in the conversation history.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique identifier |
| `content` | `String` | The actual message content |
| `embedding` | `Option<Vec<f32>>` | Vector embedding for semantic search |
| `metadata` | `MemoryMetadata` | Associated metadata |

### Methods

#### `new(content: String, metadata: MemoryMetadata) -> Self`

Creates a new `MemoryEntry` with a generated UUID and no embedding.

### Example

```rust
use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
use chrono::Utc;

let metadata = MemoryMetadata {
    user_id: Some("user123".to_string()),
    conversation_id: None,
    role: MemoryRole::User,
    timestamp: Utc::now(),
    tokens: None,
    importance: None,
};

let entry = MemoryEntry::new("Hello world".to_string(), metadata);
```

## Serialization

All types implement `Serialize` and `Deserialize`, allowing easy JSON serialization:

```rust
use serde_json;

let json = serde_json::to_string(&entry)?;
let deserialized: MemoryEntry = serde_json::from_str(&json)?;
```
