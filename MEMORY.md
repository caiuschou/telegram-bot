# Memory（记忆管理）

为 dbot 提供对话记忆管理：长期记忆、短期上下文与语义检索。

## 概述

- **长期记忆**：历史对话向量化存储，跨会话检索
- **短期上下文**：当前对话窗口管理
- **智能检索**：按当前问题检索相关历史
- **个性化记忆**：用户偏好与重要信息

## 核心概念

| 概念 | 说明 |
|------|------|
| **MemoryEntry** | 单条对话数据：id、content、embedding、metadata（user_id、conversation_id、role、timestamp 等） |
| **MemoryStore** | 存储接口：add / get / update / delete、search_by_user、search_by_conversation、semantic_search |
| **EmbeddingService** | 文本向量化：embed、embed_batch |

## 存储后端

| 后端 | 适用场景 |
|------|----------|
| InMemoryVectorStore | 测试、开发 |
| SQLiteVectorStore | 小到中型生产 |
| LanceVectorStore | 大规模向量检索（需 protoc） |

## 配置要点

- `MEMORY_STORE_TYPE`：memory / sqlite / lance
- `MEMORY_SQLITE_PATH`、`MEMORY_LANCE_PATH`：存储路径  
详见 [docs/rag/configuration.md](docs/rag/configuration.md)。

## 详细文档

- **[docs/rag/README.md](docs/rag/README.md)** - RAG 方案与模块结构
- **[docs/rag/memory/](docs/rag/memory/)** - 类型、存储、嵌入、使用与测试
- **[memory/README.md](memory/README.md)** - memory crate 说明（若存在）
- **[memory/LANCE_INTEGRATION.md](memory/LANCE_INTEGRATION.md)** - Lance 集成

## 运行测试

```bash
cargo test -p memory
```
