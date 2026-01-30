//! Lance 向量存储集成测试
//!
//! 验证内容：
//! - Lance 数据库初始化与表创建
//! - 数据库存储目录自动创建
//! - `MemoryEntry` 写入与读取
//! - 按用户 / 会话 ID 检索
//! - 语义向量检索
//! - 数据持久化（重启后可读）
//! - list_recent 按时间倒序返回最近 N 条

use chrono::{Duration, Utc};
use tempfile::TempDir;
use uuid::Uuid;

use memory::{MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore};
use memory_lance::LanceVectorStore;

/// Lance 向量存储验证测试
///
/// 验证点：
/// - Lance 数据库初始化 - 创建数据库连接、初始化向量表
/// - 数据库目录被正确创建
/// - 消息向量化存储到 Lance（添加 MemoryEntry）
/// - 数据持久化验证
///
/// 外部交互：
/// - 在临时目录创建 Lance 数据库
/// - 创建并写入 MemoryEntry 到数据库
/// - 验证数据库文件存在且可读取
#[tokio::test]
async fn test_lance_vector_store_verification() {
    // 1. 创建临时目录用于 Lance 数据库
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lance_db_path = temp_dir.path().join("lance_db");
    let lance_path_str = lance_db_path.to_string_lossy().to_string();

    // 2. 验证数据库目录在创建前不存在
    assert!(
        !lance_db_path.exists(),
        "Lance database directory should not exist before creation"
    );

    // 3. 创建 LanceVectorStore（会自动初始化数据库和表）
    let store: LanceVectorStore = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to create LanceVectorStore");

    // 4. 验证数据库目录被正确创建
    assert!(
        lance_db_path.exists(),
        "Lance database directory should be created after store initialization"
    );

    // 5. 验证数据库目录包含文件
    let lance_db_files = std::fs::read_dir(&lance_db_path)
        .expect("Should be able to read Lance database directory");
    let file_count = lance_db_files.count();
    assert!(
        file_count > 0,
        "Lance database should contain data files, found {} files",
        file_count
    );

    // 6. 创建测试 MemoryEntry（包含模拟的 embedding 向量）
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

    // 7. 添加 MemoryEntry 到 Lance 存储
    store
        .add(entry.clone())
        .await
        .expect("Failed to add entry to Lance store");

    // 8. 验证 MemoryEntry 可以被检索
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

    // 9. 验证用户搜索功能
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

    // 10. 验证会话搜索功能
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

    // 11. 验证语义搜索功能
    let query_embedding: Vec<f32> = (0..1536).map(|i| i as f32 / 1536.0).collect();
    let search_results: Vec<MemoryEntry> = store
        .semantic_search(&query_embedding, 10, None, None)
        .await
        .expect("Failed to perform semantic search");

    assert!(
        !search_results.is_empty(),
        "Semantic search should return results"
    );

    // 12. 验证数据持久化：重新打开数据库并读取数据
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

/// Lance 向量查询验证测试
///
/// 验证点：
/// - 语义搜索返回按相似度排序的结果
/// - 查询向量与某条记录的 embedding 最接近时，该记录应排在首位
///
/// 外部交互：
/// - 在临时目录创建 Lance 数据库并写入多条带不同向量的记录
/// - 执行 semantic_search 并断言结果顺序
#[tokio::test]
async fn test_lance_vector_query_verification() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let lance_db_path = temp_dir.path().join("lance_query_db");
    let lance_path_str = lance_db_path.to_string_lossy().to_string();

    let store: LanceVectorStore = LanceVectorStore::new(&lance_path_str)
        .await
        .expect("Failed to create LanceVectorStore");

    const DIM: usize = 1536;

    // 构造三条记录：A 首维为 1，B 第二维为 1，C 第三维为 1，其余为 0
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

    // 用与 A 相同的向量查询，最近邻应为 A
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
        results[0].id,
        entry_a.id,
        "nearest to query (same as A) should be entry A"
    );
    assert_eq!(results[0].content, "entry A");

    // 用与 B 相同的向量查询，最近邻应为 B
    let query_near_b: Vec<f32> = make_embedding(1);
    let results_b = store
        .semantic_search(&query_near_b, 3, None, None)
        .await
        .expect("semantic_search");
    assert_eq!(
        results_b[0].id,
        entry_b.id,
        "nearest to query (same as B) should be entry B"
    );
}

/// list_recent 按时间倒序返回最近 N 条
///
/// 验证点：
/// - 写入多条不同 timestamp 的记录后，list_recent(limit) 返回按时间降序的前 limit 条
/// - limit=0 返回空列表
///
/// 外部交互：临时目录创建 Lance 数据库并写入三条记录。
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
