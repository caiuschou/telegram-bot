//! Lance + ContextBuilder 三策略串联集成测试
//!
//! 使用真实 Lance 存储和智谱（Zhipu）词嵌入，验证 ContextBuilder 依次执行
//! RecentMessagesStrategy、SemanticSearchStrategy、UserPreferencesStrategy 的完整链路：
//! - 写入同一会话的带向量条目（猫/狗/汽车）及偏好表述（I like / I prefer）
//! - 断言 recent_messages / semantic_messages 分别包含最近消息与语义命中（含「猫」）
//! - 断言 user_preferences 非空且含偏好关键词
//!
//! 外部交互：
//! - 临时目录创建 Lance 数据库
//! - memory::ContextBuilder、RecentMessagesStrategy、SemanticSearchStrategy、UserPreferencesStrategy、MemoryStore、embedding::EmbeddingService
//! - 智谱开放平台 API（需环境变量 BIGMODEL_API_KEY 或 ZHIPUAI_API_KEY，未设置时跳过测试）

use chrono::Utc;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

use bigmodel_embedding::BigModelEmbedding;
use embedding::EmbeddingService;
use memory::{
    ContextBuilder, MemoryEntry, MemoryMetadata, MemoryRole, MemoryStore, RecentMessagesStrategy,
    SemanticSearchStrategy, UserPreferencesStrategy,
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

/// 验证：Lance 存储 + ContextBuilder 串联三个策略（RecentMessages + SemanticSearch + UserPreferences）
///
/// 步骤：
/// 1. 使用智谱 BigModelEmbedding 生成向量；无 API Key 时跳过
/// 2. 创建临时 Lance 库并写入：同一会话的猫/狗/汽车条目（带向量）+ 一条偏好表述 "I like pizza and I prefer tea"
/// 3. 使用 ContextBuilder 依次挂载 RecentMessagesStrategy、SemanticSearchStrategy、UserPreferencesStrategy
/// 4. 调用 build()，断言：recent_messages / semantic_messages 分别包含最近消息与语义命中（含「猫」），user_preferences 非空且含偏好关键词
#[tokio::test]
async fn test_lance_all_three_strategies_build_context() {
    let embedding = match make_zhipu_embedding() {
        Some(svc) => svc,
        None => return,
    };

    let temp_dir = TempDir::new().expect("temp dir");
    let db_path = temp_dir.path().join("lance_three_strategies_db");
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
    let content_pref = "I like pizza and I prefer tea";

    let emb_cat = embedding.embed(content_cat).await.expect("embed cat");
    let emb_dog = embedding.embed(content_dog).await.expect("embed dog");
    let emb_car = embedding.embed(content_car).await.expect("embed car");
    let emb_pref = embedding.embed(content_pref).await.expect("embed pref");

    let meta_conv = |role: MemoryRole| MemoryMetadata {
        user_id: Some("u1".to_string()),
        conversation_id: Some("c1".to_string()),
        role,
        timestamp: Utc::now(),
        tokens: Some(10),
        importance: Some(1.0),
    };

    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_cat.to_string(),
            embedding: Some(emb_cat),
            metadata: meta_conv(MemoryRole::User),
        })
        .await
        .expect("add cat");
    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_dog.to_string(),
            embedding: Some(emb_dog),
            metadata: meta_conv(MemoryRole::User),
        })
        .await
        .expect("add dog");
    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_car.to_string(),
            embedding: Some(emb_car),
            metadata: meta_conv(MemoryRole::Assistant),
        })
        .await
        .expect("add car");
    store
        .add(MemoryEntry {
            id: Uuid::new_v4(),
            content: content_pref.to_string(),
            embedding: Some(emb_pref),
            metadata: meta_conv(MemoryRole::User),
        })
        .await
        .expect("add pref");

    let store_arc: Arc<dyn MemoryStore> = Arc::new(store);

    let context = ContextBuilder::new(store_arc)
        .with_strategy(Box::new(RecentMessagesStrategy::new(10)))
        .with_strategy(Box::new(SemanticSearchStrategy::new(3, embedding, 0.0)))
        .with_strategy(Box::new(UserPreferencesStrategy::new()))
        .for_user("u1")
        .for_conversation("c1")
        .with_query("用户问：猫吃什么？")
        .build()
        .await
        .expect("build context");

    assert!(
        !context.is_empty(),
        "context should contain recent and/or semantic messages"
    );
    // RecentMessagesStrategy 应返回会话 c1 的 4 条最近消息
    assert!(
        context.recent_messages.len() >= 4,
        "recent_messages should contain at least 4 from RecentMessagesStrategy, got {}",
        context.recent_messages.len()
    );
    let recent_joined = context.recent_messages.join(" ");
    assert!(
        recent_joined.contains("关于狗") || recent_joined.contains("狗很忠诚"),
        "recent_messages should include message about 狗, got: {}",
        recent_joined
    );
    assert!(
        recent_joined.to_lowercase().contains("pizza") || recent_joined.to_lowercase().contains("tea"),
        "recent_messages should include preference (I like pizza / I prefer tea), got: {}",
        recent_joined
    );
    // SemanticSearchStrategy 应对查询「猫」返回语义相关消息
    let semantic_joined = context.semantic_messages.join(" ");
    assert!(
        semantic_joined.contains("猫"),
        "semantic_messages should include hit about 猫, got: {}",
        semantic_joined
    );
    assert!(
        context.user_preferences.is_some(),
        "user_preferences should be set by UserPreferencesStrategy"
    );
    let prefs = context.user_preferences.as_deref().unwrap();
    assert!(
        prefs.to_lowercase().contains("like") || prefs.to_lowercase().contains("prefer"),
        "user_preferences should contain preference keywords, got: {}",
        prefs
    );
}
