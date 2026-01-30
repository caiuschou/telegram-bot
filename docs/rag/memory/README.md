# Memory 子文档索引

- **概述**：MemoryEntry、MemoryStore、EmbeddingService；实现见 memory、memory-core、memory-inmemory、memory-sqlite、memory-lance、embedding。
- [Types](types.md) - 核心类型（MemoryRole、MemoryMetadata、MemoryEntry）
- [Storage](storage.md) - MemoryStore 与各实现
- [Embeddings](embeddings.md) - 嵌入服务
- [Usage](usage.md) - 使用示例
- [Testing](testing.md) - 测试说明
- [vector-search-accuracy](vector-search-accuracy.md) - 配置、阈值、成本与降级

设计：异步、trait 抽象、UUID 标识。
