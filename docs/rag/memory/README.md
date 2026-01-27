# Memory Crate Documentation

## Overview

The `memory` crate provides a flexible and extensible framework for managing conversational memory in the dbot project. It implements a RAG (Retrieval-Augmented Generation) approach to store, retrieve, and search through conversation history.

## Core Concepts

### Memory Entry

A `MemoryEntry` represents a single piece of conversational data, containing:
- **Content**: The actual text message
- **Embedding**: Vector representation for semantic search
- **Metadata**: Contextual information (user ID, conversation ID, role, timestamp, etc.)

### Memory Store

The `MemoryStore` trait defines the interface for persisting and retrieving memory entries. Implementations can use various storage backends (in-memory, SQLite, Lance, etc.).

### Embedding Service

The `EmbeddingService` trait provides methods to convert text into vector embeddings, enabling semantic similarity search.

## Architecture

```
memory/
├── types.rs      - Core type definitions
├── store.rs      - Memory storage interface
├── embedding.rs  - Embedding generation interface
└── lib.rs        - Public API exports
```

## Topics

- [Types](./types.md) - Core data types
- [Storage](./storage.md) - Memory storage implementations
- [Embeddings](./embeddings.md) - Text embedding services
- [Usage](./usage.md) - Usage examples
- [Testing](./testing.md) - Testing guide

## Design Decisions

### Why Async?

All operations are async to support non-blocking I/O operations, essential for high-performance bot applications.

### Trait-Based Design

Both `MemoryStore` and `EmbeddingService` are traits, allowing multiple implementations (in-memory, SQLite, Lance, OpenAI, etc.) and easy testing with mock implementations.

### UUID-based Identification

Memory entries use UUIDs for unique identification, preventing conflicts across distributed systems.
