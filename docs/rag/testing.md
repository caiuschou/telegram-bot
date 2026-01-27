# 测试策略

## 单元测试

### 测试范围

- 记忆存储和检索
- 向量生成准确性
- 上下文构建逻辑
- 元数据过滤

### 测试示例

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_retrieve_memory() {
        let store = create_test_store().await;

        let entry = create_test_entry(1, 1, "test content").await;
        store.add_memory(&entry).await.unwrap();

        let retrieved = store.get_user_memories(1).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].content, "test content");
    }

    #[tokio::test]
    async fn test_search_relevant() {
        let store = create_test_store().await;

        // 添加测试数据
        store.add_memory(&create_test_entry(1, 1, "我喜欢喝咖啡").await).await.unwrap();
        store.add_memory(&create_test_entry(1, 1, "我喜欢看电影").await).await.unwrap();

        // 搜索相关记忆
        let results = store.search_relevant(1, "推荐饮品", 3).await.unwrap();
        assert!(results.len() > 0);
        assert!(results[0].content.contains("咖啡"));
    }

    #[tokio::test]
    async fn test_context_builder() {
        let store = create_test_store().await;
        let builder = ContextBuilder::new(2000, true, true);

        let context = builder.build_context(1, "test", store).await.unwrap();
        assert!(!context.is_empty());
    }
}
```

## 集成测试

### 测试范围

- 完整对话流程（保存→检索→生成）
- 多轮对话上下文连贯性
- 用户偏好检索

### 测试示例

```rust
#[tokio::test]
async fn test_full_conversation_flow() {
    // 初始化系统
    let (memory_store, embedding_service, ai_bot) = setup_integration_test().await;

    // 第一轮对话
    let response1 = ai_bot.get_ai_response_with_memory(
        "我喜欢喝咖啡",
        1,
        None
    ).await.unwrap();

    // 验证记忆已保存
    let memories = memory_store.get_user_memories(1).await.unwrap();
    assert!(memories.len() >= 2); // 用户消息 + AI回复

    // 第二轮对话（应该能检索到第一轮的上下文）
    let response2 = ai_bot.get_ai_response_with_memory(
        "推荐一杯饮品",
        1,
        None
    ).await.unwrap();

    // 验证回复中包含相关上下文
    assert!(response2.contains("咖啡") || response2.contains("咖啡"));
}

#[tokio::test]
async fn test_preference_retrieval() {
    let (memory_store, _, _) = setup_integration_test().await;

    // 添加偏好
    let entry = create_test_entry(1, 1, "我喜欢喝咖啡").await
        .with_preference(true);
    memory_store.add_memory(&entry).await.unwrap();

    // 检索偏好
    let preferences = memory_store.get_preferences(1).await.unwrap();
    assert_eq!(preferences.len(), 1);
    assert!(preferences[0].metadata.is_preference);
}
```

## 性能测试

### 测试范围

- 记忆写入速度
- 检索响应时间（<200ms）
- 大量数据下的表现（10000+条记忆）

### 测试示例

```rust
#[tokio::test]
async fn test_memory_write_performance() {
    let store = create_test_store().await;
    let embedding_service = create_test_embedding_service().await;

    let start = Instant::now();

    for i in 0..1000 {
        let content = format!("测试消息 {}", i);
        let entry = create_test_entry(i, i, &content).await;
        store.add_memory(&entry).await.unwrap();
    }

    let duration = start.elapsed();
    println!("写入1000条记忆耗时: {:?}", duration);
    assert!(duration.as_millis() < 5000); // < 5秒
}

#[tokio::test]
async fn test_search_performance() {
    let store = setup_large_dataset(10000).await; // 10000条数据

    let start = Instant::now();

    for _ in 0..100 {
        let results = store.search_relevant(1, "测试查询", 10).await.unwrap();
        assert_eq!(results.len(), 10);
    }

    let duration = start.elapsed();
    let avg_time = duration.as_millis() / 100;
    println!("平均检索时间: {}ms", avg_time);
    assert!(avg_time < 200); // < 200ms
}

#[tokio::test]
async fn test_large_dataset_performance() {
    let store = setup_large_dataset(10000).await;

    // 测试写入性能
    let write_start = Instant::now();
    for i in 10000..11000 {
        let entry = create_test_entry(1, 1, &format!("消息 {}", i)).await;
        store.add_memory(&entry).await.unwrap();
    }
    let write_duration = write_start.elapsed();

    // 测试检索性能
    let search_start = Instant::now();
    let results = store.search_relevant(1, "测试", 10).await.unwrap();
    let search_duration = search_start.elapsed();

    println!("批量写入1000条: {:?}", write_duration);
    println!("检索时间: {:?}", search_duration);

    assert!(search_duration.as_millis() < 200);
}
```

## 测试覆盖率目标

- 单元测试覆盖率 > 80%
- 集成测试覆盖主要流程
- 性能测试覆盖关键路径

## 测试运行

```bash
# 运行所有测试
cargo test

# 运行单元测试
cargo test --lib

# 运行集成测试
cargo test --test '*'

# 运行性能测试
cargo test --release --test performance
```

## 测试数据管理

使用临时的测试数据库和向量存储：

```rust
fn create_test_store() -> Arc<dyn MemoryStore> {
    let temp_path = format!("/tmp/test_memory_{}", uuid::Uuid::new_v4());
    Arc::new(InMemoryVectorStore::new(temp_path))
}
```
