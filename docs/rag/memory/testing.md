# Testing Guide

This document provides guidelines and examples for testing the `memory` crate.

## Unit Tests

### Testing Types

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use memory::{MemoryEntry, MemoryMetadata, MemoryRole};
    
    #[test]
    fn test_memory_entry_creation() {
        let metadata = MemoryMetadata {
            user_id: Some("user123".to_string()),
            conversation_id: None,
            role: MemoryRole::User,
            timestamp: chrono::Utc::now(),
            tokens: None,
            importance: None,
        };
        
        let entry = MemoryEntry::new("Test content".to_string(), metadata);
        
        assert_eq!(entry.content, "Test content");
        assert_eq!(entry.metadata.role, MemoryRole::User);
        assert!(entry.embedding.is_none());
    }
    
    #[test]
    fn test_serialization() {
        let entry = MemoryEntry::new(
            "Test".to_string(),
            MemoryMetadata {
                user_id: None,
                conversation_id: None,
                role: MemoryRole::Assistant,
                timestamp: chrono::Utc::now(),
                tokens: None,
                importance: None,
            }
        );
        
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: MemoryEntry = serde_json::from_str(&json).unwrap();
        
        assert_eq!(entry.content, deserialized.content);
    }
}
```

### Testing Mock Memory Store

```rust
use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

struct MockMemoryStore {
    entries: Arc<RwLock<HashMap<uuid::Uuid, MemoryEntry>>>,
}

#[async_trait::async_trait]
impl MemoryStore for MockMemoryStore {
    async fn add(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }
    
    async fn get(&self, id: uuid::Uuid) -> Result<Option<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        Ok(entries.get(&id).cloned())
    }
    
    async fn update(&self, entry: MemoryEntry) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id, entry);
        Ok(())
    }
    
    async fn delete(&self, id: uuid::Uuid) -> Result<(), anyhow::Error> {
        let mut entries = self.entries.write().await;
        entries.remove(&id);
        Ok(())
    }
    
    async fn search_by_user(&self, user_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect();
        Ok(results)
    }
    
    async fn search_by_conversation(&self, conversation_id: &str) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        let entries = self.entries.read().await;
        let results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| e.metadata.conversation_id.as_deref() == Some(conversation_id))
            .cloned()
            .collect();
        Ok(results)
    }
    
    async fn semantic_search(&self, _query_embedding: &[f32], _limit: usize) -> Result<Vec<MemoryEntry>, anyhow::Error> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_mock_store() {
    let store = MockMemoryStore {
        entries: Arc::new(RwLock::new(HashMap::new())),
    };
    
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: chrono::Utc::now(),
        tokens: None,
        importance: None,
    };
    let entry = MemoryEntry::new("Test".to_string(), metadata);
    
    // Test add
    store.add(entry.clone()).await.unwrap();
    
    // Test get
    let found = store.get(entry.id).await.unwrap();
    assert!(found.is_some());
    
    // Test search_by_user
    let results = store.search_by_user("user123").await.unwrap();
    assert_eq!(results.len(), 1);
    
    // Test delete
    store.delete(entry.id).await.unwrap();
    let deleted = store.get(entry.id).await.unwrap();
    assert!(deleted.is_none());
}
```

### Testing Mock Embedding Service

```rust
use memory::EmbeddingService;

struct MockEmbeddingService;

#[async_trait::async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error> {
        // Generate deterministic mock embedding based on text
        let hash = text.len() as f32;
        Ok(vec![hash, hash * 2.0, hash * 3.0])
    }
    
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
}

#[tokio::test]
async fn test_mock_embedding() {
    let service = MockEmbeddingService;
    
    let embedding = service.embed("test").await.unwrap();
    assert_eq!(embedding.len(), 3);
    
    let embeddings = service.embed_batch(&["a".into(), "b".into()]).await.unwrap();
    assert_eq!(embeddings.len(), 2);
}
```

## Integration Tests

### Testing Store and Embedding Integration

```rust
#[tokio::test]
async fn test_add_with_embedding() {
    let store = MockMemoryStore {
        entries: Arc::new(RwLock::new(HashMap::new())),
    };
    let embedding_service = MockEmbeddingService;
    
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: chrono::Utc::now(),
        tokens: None,
        importance: None,
    };
    
    let mut entry = MemoryEntry::new("Hello world".to_string(), metadata);
    
    // Add embedding
    let embedding = embedding_service.embed(&entry.content).await.unwrap();
    entry.embedding = Some(embedding);
    
    // Store entry
    store.add(entry.clone()).await.unwrap();
    
    // Verify embedding is stored
    let found = store.get(entry.id).await.unwrap();
    assert!(found.unwrap().embedding.is_some());
}
```

## Testing Best Practices

### 1. Use Async Tests

Since all methods are async, use `#[tokio::test]`:

```rust
#[tokio::test]
async fn test_async_operation() {
    // ...
}
```

### 2. Use Mock Implementations

Create mock implementations for testing without external dependencies:

```rust
struct MockStore { /* ... */ }
struct MockEmbeddingService { /* ... */ }
```

### 3. Test Edge Cases

```rust
#[tokio::test]
async fn test_nonexistent_entry() {
    let store = MockMemoryStore { /* ... */ };
    let result = store.get(uuid::Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_empty_search() {
    let store = MockMemoryStore { /* ... */ };
    let results = store.search_by_user("nonexistent").await.unwrap();
    assert!(results.is_empty());
}
```

### 4. Test Error Handling

```rust
#[tokio::test]
async fn test_error_handling() {
    let store = MockMemoryStore { /* ... */ };
    // Test that errors are properly propagated
    let result = store.get(uuid::Uuid::new_v4()).await;
    assert!(result.is_ok());
}
```

## Test Organization

### Crate-level tests

```
memory/
├── tests/
│   └── types_test.rs      # MemoryEntry, MemoryMetadata, MemoryRole
└── src/
    ├── lib.rs
    ├── migration.rs
    └── context/
        ├── mod.rs
        ├── types.rs
        ├── builder.rs
        ├── utils.rs
        └── tests/           # Context module unit tests (separate directory)
            ├── mod.rs
            ├── estimate_tokens_test.rs
            ├── context_test.rs
            └── context_builder_test.rs
```

### Context module unit tests (`src/context/tests/`)

Unit tests for the context builder live in a dedicated `tests/` subdirectory under `context/`. Coverage:

| Component | File | Covered |
|-----------|------|---------|
| `estimate_tokens` | estimate_tokens_test.rs | Empty string, single char, words; min 1 token |
| `Context` | context_test.rs | format_for_model (with/without system, preferences, recent vs semantic), to_messages (roles), is_empty, exceeds_limit (true/false/equal) |
| `ContextMetadata` | context_test.rs | Built via make_context and asserted in Context |
| `ContextBuilder` | context_builder_test.rs | new, with_token_limit, for_user, for_conversation, with_query, with_strategy, with_system_message; build() aggregates strategy result and metadata |

## Running Tests

```bash
# Run all tests
cargo test -p memory

# Run specific test file
cargo test -p memory --test types_test

# Run with output
cargo test -p memory -- --nocapture

# Run only unit tests (not integration tests)
cargo test -p memory --lib

# Run only integration tests
cargo test -p memory --test '*'
```
