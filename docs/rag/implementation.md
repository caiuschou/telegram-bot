# RAG 实现计划

## 方案对比

| 特性 | Lance | 内存+SQLite |
|------|-------|-------------|
| 性能/规模 | 高，100万+ | 低，<1万 |
| 易用/部署 | 简单 | 简单 |
| 推荐 | 生产 | 原型/小规模 |

## 分阶段

| 阶段 | 目标 | 要点 |
|------|------|------|
| 1 | 基础记忆 | MemoryStore、嵌入、InMemory/SQLite、单元测试 |
| 2 | 上下文构建 | ContextBuilder、策略、token 窗口 |
| 3 | 中间件集成 | MemoryMiddleware、SyncAIHandler、集成测试 |
| 4 | Lance（可选） | memory-lance、生产向量存储 |
| 5 | 优化 | 相似度阈值、可观测性、回归集等 |

当前实现：memory、memory-strategies、memory-lance、memory-sqlite、memory-inmemory、embedding、middleware、ai-handlers 已就绪；配置与调优见 [configuration.md](configuration.md)、[memory/vector-search-accuracy.md](memory/vector-search-accuracy.md)。
