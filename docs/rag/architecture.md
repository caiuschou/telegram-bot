# RAG 架构设计

## 模块结构

- **memory**：MemoryStore / EmbeddingService trait，ContextBuilder，策略（RecentMessages、SemanticSearch、UserPreferences）；实现见 memory、memory-strategies、memory-core、embedding、memory-inmemory、memory-sqlite、memory-lance。
- **telegram-bot-ai**：TelegramBotAI，与 ai-client 集成。
- **middleware**：MemoryMiddleware（before/after 存记忆）；runner 组装链与 SyncAIHandler。

## 核心组件

1. **memory 模块**：对话存储与向量化、语义检索、近期/偏好策略、上下文窗口（TokenWindowManager）。
2. **记忆中间件**：自动保存收发消息到 store；before 阶段不注入上下文（上下文在 SyncAIHandler 内 build_memory_context 构建）。
3. **AI 集成**：SyncAIHandler 使用 ContextBuilder 构建上下文，与 LlmClient 生成回复；回复由 middleware after 存记忆。

具体类型与接口见各 crate 源码（memory、memory-strategies、ai-handlers、middleware）。
