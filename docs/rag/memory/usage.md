# 使用示例

- **创建 MemoryEntry**：MemoryMetadata（user_id、conversation_id、role、timestamp 等）+ MemoryEntry::new(content, metadata)。
- **存储与检索**：store.add(entry).await；store.get(id)、search_by_user、search_by_conversation、semantic_search。
- **语义搜索**：embedding_service.embed(query) → store.semantic_search(embedding, limit)。

更多见 memory crate 的 tests/、examples/ 与 [testing.md](testing.md)。
