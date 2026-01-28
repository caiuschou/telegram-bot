# Memory (记忆管理)

> Memory 系统为 dbot 项目提供对话记忆管理功能，支持长期记忆、短期上下文和智能检索。

## 目录

- [概述](#概述)
- [核心概念](#核心概念)
- [快速开始](#快速开始)
- [存储后端](#存储后端)
- [配置说明](#配置说明)
- [使用示例](#使用示例)
- [架构设计](#架构设计)
- [性能考虑](#性能考虑)
- [详细文档](#详细文档)

## 概述

Memory 系统实现了一个灵活的 RAG (Retrieval-Augmented Generation) 框架，用于：

- **长期记忆**: 将历史对话向量化存储，支持跨会话的记忆检索
- **短期上下文**: 管理当前对话窗口，确保上下文连贯性
- **智能检索**: 根据当前问题检索相关的历史对话片段
- **个性化记忆**: 存储和检索用户偏好、重要信息

### 主要特性

- ✅ 类型安全的记忆存储，支持灵活的元数据
- ✅ 基于 trait 的异步设计，支持多种存储后端
- ✅ 向量嵌入服务，支持语义搜索
- ✅ UUID 标识，支持分布式系统
- ✅ Serde 序列化，便于数据交换

## 核心概念

### MemoryEntry (记忆条目)

表示单个对话消息的数据结构，包含：

- **id**: UUID 标识符
- **content**: 消息文本内容
- **embedding**: 向量嵌入（可选）
- **metadata**: 元数据信息
  - `user_id`: 用户标识
  - `conversation_id`: 对话标识
  - `role`: 消息角色（User/Assistant/System）
  - `timestamp`: 时间戳
  - `tokens`: token 数量
  - `importance`: 重要性评分（0.0-1.0）

### MemoryStore (记忆存储)

定义存储和检索记忆条目的接口，提供以下方法：

- `add()`: 添加新记忆
- `get()`: 获取单个记忆
- `update()`: 更新记忆
- `delete()`: 删除记忆
- `search_by_user()`: 按用户搜索
- `search_by_conversation()`: 按对话搜索
- `semantic_search()`: 语义搜索

### EmbeddingService (嵌入服务)

提供文本向量化功能：

- `embed()`: 单文本嵌入
- `embed_batch()`: 批量文本嵌入

## 快速开始

### 基本使用

```rust
use memory::{MemoryEntry, MemoryMetadata, MemoryRole};

// 创建记忆条目
let metadata = MemoryMetadata {
    user_id: Some("user123".to_string()),
    conversation_id: None,
    role: MemoryRole::User,
    timestamp: chrono::Utc::now(),
    tokens: None,
    importance: None,
};

let entry = MemoryEntry::new("Hello world".to_string(), metadata);
```

### 使用存储服务

```rust
use memory::{MemoryStore, MemoryEntry, MemoryMetadata, MemoryRole};

async fn store_and_retrieve(store: &impl MemoryStore) -> Result<(), anyhow::Error> {
    // 创建条目
    let metadata = MemoryMetadata {
        user_id: Some("user123".to_string()),
        conversation_id: None,
        role: MemoryRole::User,
        timestamp: chrono::Utc::now(),
        tokens: None,
        importance: None,
    };
    let entry = MemoryEntry::new("My message".to_string(), metadata);

    // 存储条目
    store.add(entry.clone()).await?;

    // 检索条目
    let found = store.get(entry.id).await?;
    assert!(found.is_some());

    Ok(())
}
```

### 语义搜索

```rust
use memory::{MemoryStore, EmbeddingService};

async fn semantic_search(
    store: &impl MemoryStore,
    embedding_service: &impl EmbeddingService,
    query: &str,
) -> Result<(), anyhow::Error> {
    // 生成查询的嵌入
    let query_embedding = embedding_service.embed(query).await?;

    // 搜索相似条目
    let similar = store.semantic_search(&query_embedding, 5).await?;

    println!("Top {} results for '{}':", similar.len(), query);
    for (i, entry) in similar.iter().enumerate() {
        println!("{}. {}", i + 1, entry.content);
    }

    Ok(())
}
```

## 存储后端

Memory 系统支持多种存储后端：

| 后端 | Feature Flag | 描述 | 适用场景 |
|------|--------------|------|----------|
| `InMemoryVectorStore` | default | 内存存储 | 测试、开发 |
| `SQLiteVectorStore` | default | SQLite 持久化 | 生产环境（小规模） |
| `LanceVectorStore` | `lance` | LanceDB 向量存储 | 生产环境（大规模） |

### InMemoryVectorStore

- **优点**: 最快速度，简单易用
- **缺点**: 数据不持久，重启丢失
- **适用**: 测试、开发环境

### SQLiteVectorStore

- **优点**: 持久化存储，成熟稳定
- **缺点**: 大规模查询性能有限
- **适用**: 小到中型应用（< 100K 条目）

### LanceVectorStore

- **优点**: 高性能向量搜索，可扩展性强
- **缺点**: 需要额外依赖（protoc）
- **适用**: 大型应用（> 100K 条目）

#### 安装 LanceDB 支持

LanceDB 需要 Protocol Buffers 编译器（`protoc`）：

**Ubuntu/Debian:**
```bash
sudo apt-get install protobuf-compiler
```

**macOS:**
```bash
brew install protobuf
```

**验证:**
```bash
protoc --version
```

## 配置说明

在 `.env` 文件中配置记忆存储：

```env
# 内存存储类型
MEMORY_STORE_TYPE=memory

# SQLite 存储路径（当 MEMORY_STORE_TYPE=sqlite 时）
MEMORY_SQLITE_PATH=./data/memory.db

# Lance 存储路径（当 MEMORY_STORE_TYPE=lance 时）
MEMORY_LANCE_PATH=./data/lance
```

### 环境变量说明

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `MEMORY_STORE_TYPE` | `memory` | 存储类型：`memory`/`sqlite`/`lance` |
| `MEMORY_SQLITE_PATH` | `./data/memory.db` | SQLite 数据库路径 |
| `MEMORY_LANCE_PATH` | `./data/lance` | Lance 数据目录路径 |

## 使用示例

### 完整工作流程：添加消息并生成嵌入

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
    // 创建元数据
    let metadata = MemoryMetadata {
        user_id: Some(user_id.to_string()),
        conversation_id: Some(conversation_id.to_string()),
        role,
        timestamp: chrono::Utc::now(),
        tokens: None,
        importance: None,
    };

    // 创建条目
    let mut entry = MemoryEntry::new(content.to_string(), metadata);

    // 生成嵌入
    let embedding = embedding_service.embed(content).await?;
    entry.embedding = Some(embedding);

    // 存储条目
    store.add(entry.clone()).await?;

    Ok(entry.id)
}
```

### 检索对话上下文

```rust
use memory::{MemoryStore, EmbeddingService};

async fn get_conversation_context(
    store: &impl MemoryStore,
    embedding_service: &impl EmbeddingService,
    conversation_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<String>, anyhow::Error> {
    // 获取对话历史
    let history = store.search_by_conversation(conversation_id).await?;

    // 获取语义相关的条目
    let query_embedding = embedding_service.embed(query).await?;
    let relevant = store.semantic_search(&query_embedding, limit).await?;

    // 组合成上下文
    let context: Vec<String> = history
        .iter()
        .chain(relevant.iter())
        .map(|e| format!("[{}]: {}", e.metadata.role, e.content))
        .collect();

    Ok(context)
}
```

### 按用户搜索历史

```rust
use memory::MemoryStore;

async fn search_user_history(store: &impl MemoryStore) -> Result<(), anyhow::Error> {
    let user_id = "user123";

    // 获取用户的所有条目
    let entries = store.search_by_user(user_id).await?;

    println!("Found {} entries for user {}", entries.len(), user_id);
    for entry in entries {
        println!("- {}", entry.content);
    }

    Ok(())
}
```

### 错误处理

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

### 并发使用

```rust
use std::sync::Arc;

async fn concurrent_example(
    store: Arc<impl MemoryStore>,
) -> Result<(), anyhow::Error> {
    let store1 = Arc::clone(&store);
    let store2 = Arc::clone(&store);

    // 并发运行多个操作
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

## 架构设计

```
memory/
├── src/
│   ├── types.rs       # 核心类型定义
│   ├── store.rs       # MemoryStore trait
│   ├── embedding.rs   # EmbeddingService trait
│   └── lib.rs         # 公共 API 导出
└── Cargo.toml

crates/
├── memory-inmemory/   # 内存存储实现
├── memory-sqlite/     # SQLite 存储
└── memory-lance/      # Lance 向量存储
```

### 设计原则

1. **异步优先**: 所有操作都是异步的，支持高并发
2. **Trait 基础**: 基于 trait 的抽象，易于扩展和测试
3. **类型安全**: 充分利用 Rust 类型系统
4. **模块化**: 职责单一，易于组合

## 性能考虑

### 选择合适的存储后端

| 规模 | 推荐后端 | 说明 |
|------|----------|------|
| 小型 (< 10K) | InMemory / SQLite | 开发、测试 |
| 中型 (10K - 100K) | SQLite | 一般生产环境 |
| 大型 (> 100K) | Lance | 高性能生产环境 |

### 批处理优化

使用 `embed_batch` 处理多个文本，减少 API 调用：

```rust
let texts = vec!["msg1".into(), "msg2".into(), "msg3".into()];
let embeddings = embedding_service.embed_batch(&texts).await?;
```

### 缓存策略

对于频繁使用的文本，考虑缓存嵌入：

```rust
use std::collections::HashMap;

struct CachedEmbeddingService<S> {
    inner: S,
    cache: HashMap<String, Vec<f32>>,
}
```

## 详细文档

Memory 系统包含更详细的子文档：

### 核心文档

- **[memory/README.md](memory/README.md)** - Memory crate 总览
- **[docs/rag/memory/README.md](docs/rag/memory/README.md)** - 记忆系统概述

### 子主题文档

- **[Core Types](docs/rag/memory/types.md)** - 核心数据类型
- **[Storage](docs/rag/memory/storage.md)** - 存储实现详情
- **[Embeddings](docs/rag/memory/embeddings.md)** - 嵌入服务
- **[Usage Examples](docs/rag/memory/usage.md)** - 详细使用示例
- **[Testing Guide](docs/rag/memory/testing.md)** - 测试指南

### 相关文档

- **[docs/RAG_SOLUTION.md](docs/RAG_SOLUTION.md)** - RAG 集成方案
- **[memory/LANCE_INTEGRATION.md](memory/LANCE_INTEGRATION.md)** - Lance 集成文档

## 开发与测试

### 运行测试

```bash
# 运行所有 memory 测试
cargo test -p memory

# 运行特定测试
cargo test -p memory --test types_test

# 带输出运行
cargo test -p memory -- --nocapture
```

### 添加测试

为每个功能编写单元测试，测试文件位于 `memory/tests/` 目录：

```
memory/
├── tests/
│   ├── types_test.rs      # 类型测试
│   ├── store_test.rs      # 存储测试
│   ├── embedding_test.rs  # 嵌入测试
│   └── integration_test.rs # 集成测试
```

## 常见问题

### Q: 如何选择存储后端？

A: 根据数据规模和性能需求：
- 测试/开发：使用 `InMemoryVectorStore`
- 小规模生产：使用 `SQLiteVectorStore`
- 大规模生产：使用 `LanceVectorStore`

### Q: 如何迁移数据？

A: Memory 系统提供数据迁移工具，支持在不同存储后端之间迁移数据。

### Q: 嵌入维度的选择？

A: 根据使用场景：
- 小型（384-768）：快速、低成本，适合大多数场景
- 中型（1024-1536）：更好的语义理解
- 大型（3072+）：最高精度，高成本

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT License
