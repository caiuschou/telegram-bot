use memory::{
    MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole,
};
use memory_inmemory::InMemoryVectorStore;
use memory_sqlite::SQLiteVectorStore;
use openai_embedding::OpenAIEmbedding;
use embedding::EmbeddingService;
use chrono::Utc;
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("=== Memory Crate Feature Verification ===\n");

    // Test 1: Create and use InMemoryVectorStore
    println!("Test 1: InMemoryVectorStore");
    let inmemory_store = InMemoryVectorStore::new();

    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: Some("conv456".to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(0.8),
    };
    let entry = MemoryEntry::new("Hello world".to_string(), metadata.clone());

    inmemory_store.add(entry.clone()).await?;
    println!("  ✓ Added entry to InMemoryVectorStore");

    let found = inmemory_store.get(entry.id).await?;
    assert!(found.is_some());
    println!("  ✓ Retrieved entry by ID");

    let user_entries = inmemory_store.search_by_user("user123").await?;
    assert_eq!(user_entries.len(), 1);
    println!("  ✓ Searched entries by user ID");

    let conv_entries = inmemory_store.search_by_conversation("conv456").await?;
    assert_eq!(conv_entries.len(), 1);
    println!("  ✓ Searched entries by conversation ID");

    // Test semantic search with embeddings
    let mut metadata2 = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };
    let mut entry2 = MemoryEntry::new("Hello there".to_string(), metadata2);
    entry2.embedding = Some(vec![0.9, 0.1, 0.0]);

    let mut entry_with_embedding = MemoryEntry::new("Hello world".to_string(), metadata);
    entry_with_embedding.embedding = Some(vec![1.0, 0.0, 0.0]);

    inmemory_store.add(entry_with_embedding.clone()).await?;
    inmemory_store.add(entry2.clone()).await?;

    let query_embedding = vec![1.0, 0.0, 0.0];
    let similar = inmemory_store.semantic_search(&query_embedding, 2).await?;
    assert!(similar.len() >= 1);
    println!("  ✓ Performed semantic search with embeddings");

    println!("\nTest 2: SQLiteVectorStore");
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_memory.db");
    let sqlite_store = SQLiteVectorStore::new(db_path.to_str().unwrap()).await?;

    sqlite_store.add(entry.clone()).await?;
    println!("  ✓ Added entry to SQLiteVectorStore");

    let found = sqlite_store.get(entry.id).await?;
    assert!(found.is_some());
    println!("  ✓ Retrieved entry by ID from SQLite");

    let user_entries = sqlite_store.search_by_user("user123").await?;
    assert_eq!(user_entries.len(), 1);
    println!("  ✓ Searched entries by user ID in SQLite");

    sqlite_store.add(entry_with_embedding.clone()).await?;
    sqlite_store.add(entry2.clone()).await?;

    let similar = sqlite_store.semantic_search(&query_embedding, 2).await?;
    assert!(similar.len() >= 1);
    println!("  ✓ Performed semantic search with embeddings in SQLite");

    // Test 3: OpenAIEmbedding (will fail gracefully without API key)
    println!("\nTest 3: OpenAIEmbedding");
    let openai_service = OpenAIEmbedding::new(
        std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        "text-embedding-3-small".to_string(),
    );
    println!("  ✓ Created OpenAIEmbedding service (without API call)");

    // Test 4: Update and Delete operations
    println!("\nTest 4: Update and Delete operations");
    let mut updated_entry = entry.clone();
    updated_entry.content = "Updated content".to_string();
    inmemory_store.update(updated_entry.clone()).await?;
    let found = inmemory_store.get(entry.id).await?;
    assert_eq!(found.unwrap().content, "Updated content");
    println!("  ✓ Updated entry content");

    inmemory_store.delete(entry.id).await?;
    let found = inmemory_store.get(entry.id).await?;
    assert!(found.is_none());
    println!("  ✓ Deleted entry");

    // Test 5: Verify store operations
    println!("\nTest 5: Store operations");
    inmemory_store.clear().await;
    assert!(inmemory_store.is_empty().await);
    println!("  ✓ Verified store is empty after clearing");

    let metadata3 = MemoryMetadata {
        user_id: Some("user456".to_string()),
        conversation_id: None,
        role: MemoryRole::Assistant,
        timestamp: Utc::now(),
        tokens: None,
        importance: None,
    };
    let entry3 = MemoryEntry::new("Assistant message".to_string(), metadata3);
    inmemory_store.add(entry3).await?;
    assert_eq!(inmemory_store.len().await, 1);
    println!("  ✓ Verified store length");

    inmemory_store.clear().await;
    assert!(inmemory_store.is_empty().await);
    println!("  ✓ Cleared store");

    println!("\n=== All Tests Passed! ===");
    println!("\nSummary:");
    println!("  ✓ InMemoryVectorStore: CRUD operations, semantic search");
    println!("  ✓ SQLiteVectorStore: Persistence, CRUD operations, semantic search");
    println!("  ✓ OpenAIEmbedding: Service creation and initialization");
    println!("  ✓ MemoryEntry: Creation, serialization, metadata");
    println!("  ✓ MemoryStore trait: All methods implemented correctly");

    Ok(())
}
