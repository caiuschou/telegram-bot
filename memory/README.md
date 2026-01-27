# Memory Crate

The `memory` crate provides a flexible and extensible framework for managing conversational memory in the dbot project.

## Features

- **Type-safe memory storage** with flexible metadata
- **Async trait-based design** for multiple storage backends
- **Embedding service** for semantic search
- **UUID-based identification** for distributed systems
- **Serde serialization** for easy data exchange

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
memory = { path = "../memory" }
```

## Quick Start

```rust
use memory::{MemoryEntry, MemoryMetadata, MemoryRole};

// Create a memory entry
let metadata = MemoryMetadata {
    user_id: Some("user123".to_string()),
    conversation_id: None,
    role: MemoryRole::User,
    timestamp: chrono::Utc::now(),
    tokens: None,
    importance: None,
};

let entry = MemoryEntry::new("Hello world".to_string(), metadata);
```

## Documentation

For detailed documentation, see:
- [Types](./docs/rag/memory/types.md)
- [Storage](./docs/rag/memory/storage.md)
- [Embeddings](./docs/rag/memory/embeddings.md)
- [Usage Examples](./docs/rag/memory/usage.md)
- [Testing Guide](./docs/rag/memory/testing.md)

## Development Status

This crate is under active development as part of the RAG integration project. See the [Development Plan](./DEVELOPMENT_PLAN.md) for progress.
