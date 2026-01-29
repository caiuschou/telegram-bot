# Memory Storage

This document describes the memory storage interface and implementations.

## MemoryStore Trait

The `MemoryStore` trait defines the interface for storing and retrieving memory entries.

### Required Methods

#### `add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>`

Adds a new memory entry to the store.

#### `get(&self, id: Uuid) -> Result<Option<MemoryEntry>, anyhow::Error>`

Retrieves a memory entry by its UUID. Returns `None` if not found.

#### `update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error>`

Updates an existing memory entry.

#### `delete(&self, id: Uuid) -> Result<(), anyhow::Error>`

Deletes a memory entry by its UUID.

#### `search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error>`

Retrieves all memory entries for a specific user.

#### `search_by_conversation(&self, conversation_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error>`

Retrieves all memory entries for a specific conversation.

#### `semantic_search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<MemoryEntry>, anyhow::Error>`

Performs semantic search using vector embeddings. Returns the top `limit` most similar entries.

### Implementations

#### Implementations

- **InMemoryVectorStore** (memory-inmemory): Simple in-memory storage for testing and development
- **SQLiteVectorStore** (memory-sqlite): Persistent storage using SQLite
- **LanceVectorStore** (memory-lance): High-performance vector storage using LanceDB; supports semantic search and is verified by integration tests with real vectors (see `memory-lance/tests/lance_vector_store_integration_test.rs` and `lance_semantic_strategy_integration_test.rs`)

### Example Usage

```rust
use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
use uuid::Uuid;

async fn example(store: &impl MemoryStore) -> Result<(), anyhow::Error> {
    // Add an entry
    let entry = MemoryEntry::new(
        "Hello world".to_string(),
        MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: chrono::Utc::now(),
            tokens: None,
            importance: None,
        }
    );
    store.add(entry).await?;
    
    // Get by ID
    let found = store.get(entry.id).await?;
    
    // Search by user
    let entries = store.search_by_user("user123").await?;
    
    // Semantic search
    let embedding = vec![0.1, 0.2, 0.3]; // Obtained from embedding service
    let similar = store.semantic_search(&embedding, 10).await?;
    
    Ok(())
}
```

## Storage Considerations

### Performance

- **In-Memory**: Fastest, but data is lost on restart
- **SQLite**: Good balance of performance and persistence
- **Lance**: Optimized for vector search at scale

### Scalability

- **Small Scale (< 10K entries)**: SQLite or in-memory
- **Medium Scale (10K - 100K entries)**: SQLite with proper indexing
- **Large Scale (> 100K entries)**: Lance or dedicated vector database

### Migration

The crate will provide migration tools to transfer data between storage backends.
