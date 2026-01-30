//! Lance vector store integration tests
//!
//! Verifies:
//! - Lance DB init and table creation
//! - DB storage directory auto-creation
//! - MemoryEntry write/read
//! - Search by user / conversation ID
//! - Semantic vector search
//! - Data persistence (readable after restart)
//! - list_recent returns N most recent entries by time

use chrono::{Duration, Utc};
use tempfile::TempDir;
use uuid::Uuid;

use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use memory_lance::LanceVectorStore;

/// Lance vector store verification test
///
/// Checks:
/// - Lance DB init (connection, vector table)
/// - DB directory created correctly
/// - Message vectorization stored to Lance (add MemoryEntry)
/// - Data persistence
///
/// External: create Lance DB in temp dir, write MemoryEntry, verify files exist and are readable.
#[tokio::test]
async fn test_lance_vector_store_verification() {
    // 1. Create temp dir for Lance DB
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lance_db_path = temp_dir.path().join("lance_db");
    let lance_path_str = lance_db_path.to_string_lossy().to_string();

    // 2. DB dir should not exist before creation
    assert!(
        !lance_db_path.exists(),
        "Lance database directory should not exist before creation"
    );

    // 3. Create LanceVectorStore (auto-inits DB and table)
    let store: LanceVectorStore = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to create LanceVectorStore");

    // 4. DB dir created correctly
    assert!(
        lance_db_path.exists(),
        "Lance database directory should be created after store initialization"
    );

    // 5. DB dir contains files
    let lance_db_files = std::fs::read_dir(&lance_db_path)
        .expect("Should be able to read Lance database directory");
    let file_count = lance_db_files.count();
    assert!(
        file_count > 0,
        "Lance database should contain data files, found {} files",
        file_count
    );

    // 6. Create test MemoryEntry (mock embedding vector)
    let test_content = "Test message for Lance vector store";
    let test_embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();

    let metadata = MemoryMetadata {
        user_id: Some("test_user_123".to_string()),
        conversation_id: Some("test_conversation_456".to_string()),
        role: MemoryRole::User,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(1.0),
    };

    let entry = MemoryEntry {
        id: Uuid::new_v4(),
        content: test_content.to_string(),
        embedding: Some(test_embedding),
        metadata,
    };

    // 7. Add MemoryEntry to Lance store
    store
        .add(entry.clone())
        .await
        .expect("Failed to add entry to Lance store");

    // 8. MemoryEntry is retrievable
    let retrieved: Option<MemoryEntry> = store
        .get(entry.id)
        .await
        .expect("Failed to get entry from Lance store");

    assert!(
        retrieved.is_some(),
        "MemoryEntry should be retrievable after adding"
    );

    let retrieved_entry = retrieved.unwrap();
    assert_eq!(
        retrieved_entry.id, entry.id,
        "Retrieved entry should have the same ID"
    );
    assert_eq!(
        retrieved_entry.content, entry.content,
        "Retrieved entry should have the same content"
    );
    assert!(
        retrieved_entry.embedding.is_some(),
        "Retrieved entry should have embedding"
    );

    // 9. User search
    let user_entries: Vec<MemoryEntry> = store
        .search_by_user("test_user_123")
        .await
        .expect("Failed to search by user");

    assert!(
        !user_entries.is_empty(),
        "Should find entries for test_user_123"
    );
    assert!(
        user_entries.iter().any(|e| e.id == entry.id),
        "Search results should include the added entry"
    );

    // 10. Conversation search
    let conversation_entries: Vec<MemoryEntry> = store
        .search_by_conversation("test_conversation_456")
        .await
        .expect("Failed to search by conversation");

    assert!(
        !conversation_entries.is_empty(),
        "Should find entries for test_conversation_456"
    );
    assert!(
        conversation_entries.iter().any(|e| e.id == entry.id),
        "Search results should include the added entry"
    );

    // 11. Semantic search
    let query_embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
    let search_results: Vec<(f32, MemoryEntry)> = store
        .semantic_search(&query_embedding, 10, None, None)
        .await
        .expect("Failed to perform semantic search");

    assert!(
        !search_results.is_empty(),
        "Semantic search should return results"
    );

    // 12. Persistence: reopen DB and read
    let store2: LanceVectorStore = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to reopen LanceVectorStore");

    let retrieved_after_reopen: Option<MemoryEntry> = store2
        .get(entry.id)
        .await
        .expect("Failed to get entry after reopening store");

    assert!(
        retrieved_after_reopen.is_some(),
        "MemoryEntry should be retrievable after reopening the store (data persistence)"
    );
}

/// Lance vector query verification test
///
/// Checks:
/// - Semantic search returns results ordered by similarity
/// - When query vector is closest to a record's embedding, that record is first
///
/// External: create Lance DB in temp dir, write entries with different vectors, run semantic_search and assert order
#[tokio::test]
async fn test_lance_vector_query_verification() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lance_db_path = temp_dir.path().join("lance_query_db");
    let lance_path_str = lance_db_path.to_string_lossy().to_string();

    let store: LanceVectorStore = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to create LanceVectorStore");

    const DIM: usize = 1536;

    // Three records: A dim 0 = 1, B dim 1 = 1, C dim 2 = 1, rest 0
    let make_embedding = |i: usize| {
        let mut v = vec![0.0f32; DIM];
        v[i] = 1.0;
        v
    };

    let meta = |role: MemoryRole| MemoryMetadata {
        user_id: Some("u1".to_string()),
        conversation_id: Some("c1".to_string()),
        role,
        timestamp: Utc::now(),
        tokens: Some(5),
        importance: Some(1.0),
    };

    let entry_a = MemoryEntry {
        id: Uuid::new_v4(),
        content: "entry A".to_string(),
        embedding: Some(make_embedding(0)),
        metadata: meta(MemoryRole::User),
    };
    let entry_b = MemoryEntry {
        id: Uuid::new_v4(),
        content: "entry B".to_string(),
        embedding: Some(make_embedding(1)),
        metadata: meta(MemoryRole::User),
    };
    let entry_c = MemoryEntry {
        id: Uuid::new_v4(),
        content: "entry C".to_string(),
        embedding: Some(make_embedding(2)),
        metadata: meta(MemoryRole::Assistant),
    };

    store.add(entry_a.clone()).await.expect("add A");
    store.add(entry_b.clone()).await.expect("add B");
    store.add(entry_c.clone()).await.expect("add C");

    // Query with same vector as A; nearest should be A
    let query_near_a: Vec<f32> = make_embedding(0);
    let results = store
        .semantic_search(&query_near_a, 3, None, None)
        .await
        .expect("semantic_search");

    assert!(
        !results.is_empty(),
        "semantic_search should return at least one result"
    );
    assert_eq!(
        results[0].1.id,
        entry_a.id,
        "nearest to query (same as A) should be entry A"
    );
    assert_eq!(results[0].1.content, "entry A");

    // Query with same vector as B; nearest should be B
    let query_near_b: Vec<f32> = make_embedding(1);
    let results_b = store
        .semantic_search(&query_near_b, 3, None, None)
        .await
        .expect("semantic_search");
    assert_eq!(
        results_b[0].1.id,
        entry_b.id,
        "nearest to query (same as B) should be entry B"
    );
}

/// Semantic search regression: 3 golden cases (query -> expected hit)
///
/// One-hot vector fixtures, no external API, CI-stable.
/// When query vector matches a record's embedding, that record is first.
///
/// | Case | Query vector | Expected content |
/// | 1 | dim 0 = 1, rest 0 | "entry A" |
/// | 2 | dim 1 = 1, rest 0 | "entry B" |
/// | 3 | dim 2 = 1, rest 0 | "entry C" |
#[tokio::test]
async fn test_semantic_search_regression_golden_cases() {
    let temp_dir = TempDir::new().expect("temp dir");
    let lance_db_path = temp_dir.path().join("lance_regression_db");
    let lance_path_str = lance_db_path.to_string_lossy().to_string();

    let store: LanceVectorStore = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("create LanceVectorStore");

    const DIM: usize = 1536;
    let make_embedding = |i: usize| {
        let mut v = vec![0.0f32; DIM];
        v[i] = 1.0;
        v
    };

    let meta = |role: MemoryRole| MemoryMetadata {
        user_id: Some("u1".to_string()),
        conversation_id: Some("c1".to_string()),
        role,
        timestamp: Utc::now(),
        tokens: Some(5),
        importance: Some(1.0),
    };

    let entry_a = MemoryEntry {
        id: Uuid::new_v4(),
        content: "entry A".to_string(),
        embedding: Some(make_embedding(0)),
        metadata: meta(MemoryRole::User),
    };
    let entry_b = MemoryEntry {
        id: Uuid::new_v4(),
        content: "entry B".to_string(),
        embedding: Some(make_embedding(1)),
        metadata: meta(MemoryRole::User),
    };
    let entry_c = MemoryEntry {
        id: Uuid::new_v4(),
        content: "entry C".to_string(),
        embedding: Some(make_embedding(2)),
        metadata: meta(MemoryRole::Assistant),
    };

    store.add(entry_a).await.expect("add A");
    store.add(entry_b).await.expect("add B");
    store.add(entry_c).await.expect("add C");

    let golden_cases: [(usize, &str); 3] = [
        (0, "entry A"),
        (1, "entry B"),
        (2, "entry C"),
    ];

    for (query_dim, expect_content_substr) in golden_cases {
        let query_vec = make_embedding(query_dim);
        let results = store
            .semantic_search(&query_vec, 3, None, None)
            .await
            .expect("semantic_search");
        assert!(
            !results.is_empty(),
            "regression case query_dim={}: should return at least one result",
            query_dim
        );
        let top_content = &results[0].1.content;
        assert!(
            top_content.contains(expect_content_substr),
            "regression case query_dim={}: expected top result to contain {:?}, got {:?}",
            query_dim,
            expect_content_substr,
            top_content
        );
    }
}

/// list_recent returns N most recent entries by time (desc)
///
/// Checks:
/// - After writing entries with different timestamps, list_recent(limit) returns first limit by time desc
/// - limit=0 returns empty list
///
/// External: temp dir Lance DB with three records.
#[tokio::test]
async fn test_lance_list_recent_returns_ordered_by_timestamp_desc() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lance_db_path = temp_dir.path().join("lance_list_recent_db");
    let lance_path_str = lance_db_path.to_string_lossy().to_string();

    let store: LanceVectorStore = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to create LanceVectorStore");

    const DIM: usize = 1536;
    let base_time = Utc::now() - Duration::seconds(10);

    let make_entry = |content: &str, secs_after_base: i64| {
        let metadata = MemoryMetadata {
            user_id: Some("u1".to_string()),
            conversation_id: Some("c1".to_string()),
            role: MemoryRole::User,
            timestamp: base_time + Duration::seconds(secs_after_base),
            tokens: Some(5),
            importance: Some(1.0),
        };
        MemoryEntry {
            id: Uuid::new_v4(),
            content: content.to_string(),
            embedding: Some(vec![0.1f32; DIM]),
            metadata,
        }
    };

    let e1 = make_entry("oldest", 0);
    let e2 = make_entry("middle", 5);
    let e3 = make_entry("newest", 9);

    store.add(e1.clone()).await.expect("add e1");
    store.add(e2.clone()).await.expect("add e2");
    store.add(e3.clone()).await.expect("add e3");

    let recent = store.list_recent(2).await.expect("list_recent(2)");
    assert_eq!(recent.len(), 2, "list_recent(2) should return 2 entries");
    assert_eq!(
        recent[0].content, "newest",
        "first should be newest by timestamp"
    );
    assert_eq!(
        recent[1].content, "middle",
        "second should be middle by timestamp"
    );

    let recent_all = store.list_recent(10).await.expect("list_recent(10)");
    assert_eq!(
        recent_all.len(), 3,
        "list_recent(10) should return all 3 entries"
    );
    assert_eq!(recent_all[0].content, "newest");
    assert_eq!(recent_all[1].content, "middle");
    assert_eq!(recent_all[2].content, "oldest");

    let empty = store.list_recent(0).await.expect("list_recent(0)");
    assert!(empty.is_empty(), "list_recent(0) should return empty");
}
