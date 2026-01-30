# 使用示例

- **初始化**：BotConfig 从环境变量加载；runner 根据 MEMORY_STORE_TYPE 创建 store（memory/sqlite/lance），可选 recent_store（MEMORY_RECENT_USE_SQLITE），与 embedding 一起传入 MemoryMiddleware 和 SyncAIHandler。
- **代码入口**：`telegram-bot/src/runner.rs`（组装链）、`ai-handlers/src/sync_ai_handler.rs`（上下文构建与 LLM 调用）、`memory/src/context/builder.rs`（ContextBuilder）。
- **Lance**：见 [LANCE_USAGE.md](LANCE_USAGE.md)；API 调研见 [LANCE_API_RESEARCH.md](LANCE_API_RESEARCH.md)。

更多示例见各 crate 的 tests/ 与 examples/。
