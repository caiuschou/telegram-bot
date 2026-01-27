# Usage Examples

This document provides practical examples of using the `memory` crate.

## Basic Usage

### Creating a Memory Entry

```rust
use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
use chrono::Utc;

fn create_entry() -> MemoryEntry {
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: Some("conv456".to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(0.8),
    };
    
    MemoryEntry::new("Hello, how are you?".to_string(), metadata)
}
```

### Storing and Retrieving Entries

```rust
use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};

async fn store_and_retrieve(store: &impl MemoryStore) -> Result<(), anyhow::Error> {
    // Create entry
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: chrono::Utc::now(),
        tokens: None,
        importance: None,
    };
    let entry = MemoryEntry::new("My message".to_string(), metadata);
    
    // Store entry
    store.add(entry.clone()).await?;
    
    // Retrieve entry
    let found = store.get(entry.id).await?;
    assert!(found.is_some());
    
    Ok(())
}
```

### Searching by User

```rust
use memory::MemoryStore;

async fn search_user_history(store: &impl MemoryStore) -> Result<(), anyhow::Error> {
    let user_id = "user123";
    
    // Get all entries for a user
    let entries = store.search_by_user(user_id).await?;
    
    println!("Found {} entries for user {}", entries.len(), user_id);
    for entry in entries {
        println!("- {}", entry.content);
    }
    
    Ok(())
}
```

### Searching by Conversation

```rust
use memory::MemoryStore;

async fn search_conversation(store: &impl MemoryStore) -> Result<(), anyhow::Error> {
    let conversation_id = "conv456";
    
    // Get all entries for a conversation
    let entries = store.search_by_conversation(conversation_id).await?;
    
    println!("Conversation {} has {} messages:", conversation_id, entries.len());
    for entry in entries {
        println!("- [{}] {}", entry.metadata.role, entry.content);
    }
    
    Ok(())
}
```

### Semantic Search

```rust
use memory::{MemoryStore, EmbeddingService};

async fn semantic_search(
    store: &impl MemoryStore,
    embedding_service: &impl EmbeddingService,
    query: &str,
) -> Result<(), anyhow::Error> {
    // Generate embedding for query
    let query_embedding = embedding_service.embed(query).await?;
    
    // Search for similar entries
    let similar = store.semantic_search(&query_embedding, 5).await?;
    
    println!("Top {} results for '{}':", similar.len(), query);
    for (i, entry) in similar.iter().enumerate() {
        println!("{}. {}", i + 1, entry.content);
    }
    
    Ok(())
}
```

## Complete Workflow

### Adding Messages with Embeddings

```rust
use memory::{MemoryStore, EmbeddingService, MemoryEntry, MemoryMetadata, MemoryRole};

async fn add_message_with_embedding(
    store: &impl MemoryStore,
    embedding_service: &impl EmbeddingService,
    user_id: &str,
    conversation_id: &str,
    content: &str,
    role: MemoryRole,
) -> Result<Uuid, anyhow::Error> {
    // Create metadata
    let metadata = MemoryMetadata {
        user_id: Some(user_id.to_string()),
        conversation_id: Some(conversation_id.to_string()),
        role,
        timestamp: chrono::Utc::now(),
        tokens: None,
        importance: None,
    };
    
    // Create entry
    let mut entry = MemoryEntry::new(content.to_string(), metadata);
    
    // Generate embedding
    let embedding = embedding_service.embed(content).await?;
    entry.embedding = Some(embedding);
    
    // Store entry
    store.add(entry.clone()).await?;
    
    Ok(entry.id)
}
```

### Retrieving Context for AI

```rust
use memory::{MemoryStore, EmbeddingService};

async fn get_conversation_context(
    store: &impl MemoryStore,
    embedding_service: &impl EmbeddingService,
    conversation_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<String>, anyhow::Error> {
    // Get conversation history
    let history = store.search_by_conversation(conversation_id).await?;
    
    // Get semantically relevant entries
    let query_embedding = embedding_service.embed(query).await?;
    let relevant = store.semantic_search(&query_embedding, limit).await?;
    
    // Combine into context
    let context: Vec<String> = history
        .iter()
        .chain(relevant.iter())
        .map(|e| format!("[{}]: {}", e.metadata.role, e.content))
        .collect();
    
    Ok(context)
}
```

## Error Handling

All methods return `Result<T, anyhow::Error>`:

```rust
use memory::MemoryStore;
use anyhow::Result;

async fn safe_retrieve(store: &impl MemoryStore, id: uuid::Uuid) -> Result<Option<String>> {
    match store.get(id).await {
        Ok(Some(entry)) => Ok(Some(entry.content)),
        Ok(None) => Ok(None),
        Err(e) => {
            eprintln!("Error retrieving entry: {}", e);
            Err(e)
        }
    }
}
```

## Concurrency

The traits require `Send + Sync`, enabling safe concurrent use:

```rust
use std::sync::Arc;

async fn concurrent_example(
    store: Arc<impl MemoryStore>,
) -> Result<(), anyhow::Error> {
    let store1 = Arc::clone(&store);
    let store2 = Arc::clone(&store);
    
    // Run multiple operations concurrently
    let task1 = tokio::spawn(async move {
        store1.search_by_user("user123").await
    });
    
    let task2 = tokio::spawn(async move {
        store2.search_by_user("user456").await
    });
    
    let (result1, result2) = tokio::try_join!(task1, task2)?;
    
    Ok(())
}
```
