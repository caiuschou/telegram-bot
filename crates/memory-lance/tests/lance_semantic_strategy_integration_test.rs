//! Lance + SemanticSearchStrategy 集成测试
//!
//! 使用真实 Lance 存储和智谱（Zhipu）词嵌入验证完整策略链路：
//! - 使用 BigModelEmbedding（智谱 embedding-2）为测试数据生成向量并写入 Lance
//! - SemanticSearchStrategy 使用同一智谱服务对查询生成向量并做语义检索
//! - 验证查询「猫」时返回与「关于猫」语义最近的一条消息
//!
//! 外部交互：
//! - 临时目录创建 Lance 数据库
//! - memory::SemanticSearchStrategy、memory::MemoryStore、embedding::EmbeddingService
//! - 智谱开放平台 API（需环境变量 BIGMODEL_API_KEY 或 ZHIPUAI_API_KEY，未设置时跳过测试）

use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

use bigmodel_embedding::BigModelEmbedding;
use embedding::EmbeddingService;
use memory::{
    ContextStrategy, MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, SemanticSearchStrategy,
    StrategyResult,
};
use memory_lance::{LanceConfig, LanceVectorStore};

/// 智谱 embedding-2 模型向量维度
const DIM: usize = 1024;

/// 从环境变量获取智谱 API Key；未设置时返回 None，测试将跳过。
fn zhipu_api_key() -> Option<String> {
    std::env::var("BIGMODEL_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("ZHIPUAI_API_KEY").ok().filter(|s| !s.is_empty()))
}

/// 创建智谱词嵌入服务（embedding-2），无 API Key 时返回 None。
fn make_zhipu_embedding() -> Option<Arc<BigModelEmbedding>> {
    zhipu_api_key().map(|key| Arc::new(BigModelEmbedding::new(key, "embedding-2".to_string())))
}

fn meta(role: MemoryRole) -> MemoryMetadata {
    MemoryMetadata {
        user_id: Some("u1".to_string()),
        conversation_id: Some("c1".to_string()),
        role,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(1.0),
    }
}

/// 验证：Lance 存储 + SemanticSearchStrategy + 智谱词嵌入
///
/// 步骤：
/// 1. 使用智谱 BigModelEmbedding（embedding-2）生成向量；无 API Key 时跳过
/// 2. 创建临时 Lance 库并写入三条带智谱向量的 MemoryEntry（猫、狗、汽车）
/// 3. 对查询「用户问：猫吃什么？」执行 build_context，断言返回的消息中包含「关于猫」且为最近邻
#[tokio::test]
async fn test_lance_semantic_strategy_returns_relevant_message() {
    let embedding = match make_zhipu_embedding() {
        Some(svc) => svc,
        None => return,
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("lance_semantic_db");
    let config = LanceConfig {
        db_path: db_path.to_string_lossy().to_string(),
        table_name: "memories".to_string(),
        embedding_dim: DIM,
        ..Default::default()
    };

    let store = LanceVectorStore::with_config(config)
        .await
        .expect("create LanceVectorStore");

    let content_cat = "关于猫的讨论：猫是可爱的宠物。";
    let content_dog = "关于狗的讨论：狗很忠诚。";
    let content_car = "关于汽车的讨论：电动汽车很环保。";

    let emb_cat = embedding.embed(content_cat).await.expect("embed cat");
    let emb_dog = embedding.embed(content_dog).await.expect("embed dog");
    let emb_car = embedding.embed(content_car).await.expect("embed car");

    let entry_cat = MemoryEntry {
        id: Uuid::new_v4(),
        content: content_cat.to_string(),
        embedding: Some(emb_cat),
        metadata: meta(MemoryRole::User),
    };
    let entry_dog = MemoryEntry {
        id: Uuid::new_v4(),
        content: content_dog.to_string(),
        embedding: Some(emb_dog),
        metadata: meta(MemoryRole::User),
    };
    let entry_car = MemoryEntry {
        id: Uuid::new_v4(),
        content: content_car.to_string(),
        embedding: Some(emb_car),
        metadata: meta(MemoryRole::Assistant),
    };

    store.add(entry_cat).await.expect("add cat");
    store.add(entry_dog).await.expect("add dog");
    store.add(entry_car).await.expect("add car");

    let strategy = SemanticSearchStrategy::new(3, embedding, 0.0);
    let result = strategy
        .build_context(&store, &None, &None, &Some("用户问：猫吃什么？".to_string()))
        .await
        .expect("build_context");

    let messages = match &result {
        StrategyResult::Messages { messages: m, .. } => m,
        StrategyResult::Empty => panic!("expected Messages, got Empty"),
        StrategyResult::Preferences(_) => panic!("expected Messages, got Preferences"),
    };

    assert!(
        !messages.is_empty(),
        "semantic search with Lance should return at least one message"
    );

    let first = &messages[0];
    assert!(
        first.contains("猫"),
        "nearest to query '猫' should be the cat entry, got: {}",
        first
    );
    assert!(
        first.contains("关于猫"),
        "first message should be the cat discussion, got: {}",
        first
    );
}

/// 验证：无查询时策略返回 Empty
#[tokio::test]
async fn test_lance_semantic_strategy_empty_query_returns_empty() {
    let embedding = match make_zhipu_embedding() {
        Some(svc) => svc,
        None => return,
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("lance_empty_query_db");
    let config = LanceConfig {
        db_path: db_path.to_string_lossy().to_string(),
        table_name: "memories".to_string(),
        embedding_dim: DIM,
        ..Default::default()
    };

    let store = LanceVectorStore::with_config(config)
        .await
        .expect("create LanceVectorStore");

    let content = "一条消息";
    let emb = embedding.embed(content).await.expect("embed");
    let entry = MemoryEntry {
        id: Uuid::new_v4(),
        content: content.to_string(),
        embedding: Some(emb),
        metadata: meta(MemoryRole::User),
    };
    store.add(entry).await.expect("add");

    let strategy = SemanticSearchStrategy::new(5, embedding, 0.0);
    let result = strategy
        .build_context(&store, &None, &None, &None)
        .await
        .expect("build_context");

    assert!(
        matches!(result, StrategyResult::Empty),
        "no query should yield Empty"
    );
}
