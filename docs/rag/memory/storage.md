# Memory 存储

## MemoryStore Trait

- **add** / **get** / **update** / **delete**
- **search_by_user** / **search_by_conversation**
- **semantic_search**(query_embedding, limit) → Vec<MemoryEntry>（部分实现带分数）

## 实现

| 实现 | Crate | 说明 |
|------|-------|------|
| InMemoryVectorStore | memory-inmemory | 内存，测试/开发 |
| SQLiteVectorStore | memory-sqlite | SQLite；支持 search_by_* 与 semantic_search |
| LanceVectorStore | memory-lance | LanceDB，生产向量检索 |

详见各 crate 与 memory-core。
