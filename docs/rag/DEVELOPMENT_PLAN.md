# RAG 功能开发计划

## 概述

- **目标**：为 dbot 集成 RAG（对话记忆、上下文构建）。
- **技术栈**：Rust、OpenAI/智谱、Lance/SQLite。
- **总工期**：基础 4–7 天；含 Lance 约 9–13 天。

## 阶段摘要

| 阶段 | 目标 | 状态 |
|------|------|------|
| 1 | 基础记忆（MemoryStore、嵌入、InMemory/SQLite、单元测试） | 已完成 |
| 2 | 上下文构建（ContextBuilder、策略、token 窗口） | 已完成 |
| 3 | 中间件集成（MemoryMiddleware、SyncAIHandler、集成测试） | 已完成 |
| 4 | Lance 集成（可选） | 已完成 |
| 5 | 优化（相似度阈值、可观测性、回归集等） | 部分完成，见 vector-search-accuracy-plan |

详细任务表与验收见 git 历史；当前实现见各 crate 与 [implementation.md](implementation.md)。
