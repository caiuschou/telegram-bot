# 数据流

## 消息处理流程

1. 用户发消息 → HandlerChain 处理。
2. MemoryMiddleware before：保存用户消息到 store（及可选 recent_store）。
3. SyncAIHandler：build_memory_context（ContextBuilder + 策略）→ 构建 messages → LlmClient 生成回复 → 发/编辑消息；返回 Reply。
4. MemoryMiddleware after：保存 AI 回复到 store（及 recent_store）。
5. 回复落库（消息持久化在 storage）。

## 上下文构建

ContextBuilder 按策略收集：RecentMessagesStrategy（近期 N 条）、SemanticSearchStrategy（语义 Top-K，可选相似度阈值）、UserPreferencesStrategy；TokenWindowManager 控制 token 上限。详见 [context_builder_design.md](context_builder_design.md)、[context-retrieval-before-reply.md](context-retrieval-before-reply.md)。
