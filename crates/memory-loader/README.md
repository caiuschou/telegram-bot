# Memory Loader

从 SQLite 消息数据加载到 LanceDB 向量库：读取消息 → 转 MemoryEntry → 生成 embedding → 写入 Lance。

## 使用（通过 dbot-cli）

```bash
./target/release/dbot load
./target/release/dbot load --batch-size 100
```

## 配置

- **LoadConfig**：database_url、lance_db_path、embedding（EmbeddingConfig）、batch_size。
- **EmbeddingConfig**：provider（OpenAI/Zhipuai）、model 可选、openai_api_key、bigmodel_api_key。
- 环境变量：DATABASE_URL、LANCE_DB_PATH、EMBEDDING_PROVIDER、OPENAI_API_KEY/BIGMODEL_API_KEY、EMBEDDING_MODEL（可选）。

## 流程

1. 连接 SQLite（MessageRepository）、Lance（LanceVectorStore）、EmbeddingService。
2. 批量：读 batch_size 条 → convert → embed_batch → 写入 Lance；循环至结束。
3. 返回 LoadResult（total、loaded、elapsed_secs）。

## 数据转换

MessageRecord → MemoryEntry：id、content、metadata（user_id、conversation_id、role、timestamp）；role 由 direction 映射（received→User、sent→Assistant）。见 memory-loader/src/converter.rs。

## 依赖

storage、memory-lance、memory-core、embedding、openai-embedding（或 bigmodel-embedding）。智谱支持见 EmbeddingProvider::Zhipuai 与 .env 配置；开发计划已完成。
