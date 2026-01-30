# Crate 与文件索引

## 原则：每个 Crate 尽量简单

- **单一职责**：一个 crate 只做一类事（如核心类型、存储、记忆、链式处理等）。
- **少依赖**：尽量只依赖本 workspace 内必要 crate，避免把「可选能力」写进核心 crate。
- **文件清晰**：每个源文件职责明确，在下方「文件与描述」中可查。

---

## 1. dbot-core

| 项目 | 说明 |
|------|------|
| **路径** | `dbot-core/` |
| **描述** | Bot 框架核心：领域类型（User、Chat、Message）、Handler/Middleware/Bot 抽象、错误与日志。与具体传输（Telegram）无关，供上层与 handler-chain 使用。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口与公开 API 再导出。 |
| `src/types.rs` | 核心类型：User、Chat、Message、MessageDirection、HandlerResponse；ToCoreUser/ToCoreMessage；Handler、Middleware trait。 |
| `src/bot.rs` | Bot trait：send_message、reply_to、edit_message、send_message_and_return_id；与具体传输无关。 |
| `src/error.rs` | DbotError、HandlerError、Result 等错误定义。 |
| `src/logger.rs` | init_tracing：控制台 + 文件的双输出 tracing 初始化。 |

---

## 2. dbot-telegram

| 项目 | 说明 |
|------|------|
| **路径** | `crates/dbot-telegram/` |
| **描述** | Telegram Bot 框架层：适配器（teloxide→core）、Bot trait 实现、最小配置、REPL 运行（run_repl）。仅负责 Telegram 接入与消息链执行，不包含持久化/记忆/AI。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口；导出 TelegramMessageWrapper、TelegramUserWrapper、TelegramBotAdapter、TelegramConfig、run_repl。 |
| `src/adapters.rs` | TelegramMessageWrapper、TelegramUserWrapper：teloxide Message/User 转 dbot_core::Message/User。 |
| `src/bot_adapter.rs` | TelegramBotAdapter：包装 teloxide::Bot，实现 dbot_core::Bot（send_message、reply_to、edit_message、send_message_and_return_id）。 |
| `src/config.rs` | TelegramConfig：仅 token、api_url、log_file；from_env()、with_token()。 |
| `src/runner.rs` | run_repl(bot, handler_chain, bot_username)：get_me 写回 bot_username，REPL 内 to_core 后 chain.handle。 |

---

## 3. ai-client

| 项目 | 说明 |
|------|------|
| **路径** | `crates/ai-client/` |
| **描述** | AI 层抽象：LlmClient trait 与 OpenAILlmClient 实现。仅依赖 openai-client、prompt，无 dbot-core/teloxide。供 ai-handlers、telegram-bot-ai 使用。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | LlmClient trait（get_ai_response_with_messages、get_ai_response_stream_with_messages）；StreamChunk；OpenAILlmClient 实现（ChatMessage→OpenAI、system 前置、chat_completion/stream）。 |

---

## 4. storage

| 项目 | 说明 |
|------|------|
| **路径** | `storage/` |
| **描述** | 消息持久化：Repository trait、MessageRepository（SQLite）、消息模型与查询。供 telegram-bot 持久化中间件及 AI 响应落库使用。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口与公开 API；导出 StorageError、MessageRepository、models、Repository、SqlitePoolManager。 |
| `src/repository.rs` | Repository trait 定义（如 save、query）。 |
| `src/message_repo.rs` | MessageRepository：基于 SQLite 的 Repository 实现。 |
| `src/sqlite_pool.rs` | SQLite 连接池管理。 |
| `src/error.rs` | StorageError 等存储错误类型。 |
| `src/models/mod.rs` | 模型模块入口。 |
| `src/models/message_record.rs` | MessageRecord：单条消息记录结构。 |
| `src/models/message_query.rs` | MessageQuery：查询条件等。 |
| `src/models/message_stats.rs` | MessageStats：统计相关结构。 |
| `src/message_repo_test.rs` | MessageRepository 相关测试（仅 test）。 |

---

## 5. ai-handlers

| 项目 | 说明 |
|------|------|
| **路径** | `ai-handlers/` |
| **描述** | AI 相关 Handler：检测 @ 提及/回复机器人、同步 RAG 处理（建上下文、调 LLM、发回复）。依赖 dbot_core::Bot + ai_client::OpenAILlmClient，不依赖 teloxide/telegram-bot-ai。实现 dbot_core::Handler，供 handler-chain 挂载。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口；导出 AIDetectionHandler、AIQuery、SyncAIHandler。 |
| `src/ai_mention_detector.rs` | AIDetectionHandler、AIQuery：判断是否回复机器人或 @ 提及，将 AI 查询发往 channel。 |
| `src/sync_ai_handler.rs` | SyncAIHandler：持有 Arc<OpenAILlmClient>、Arc<dyn Bot>；链内 RAG（ContextBuilder + llm_client + bot.send_message / edit_message），返回 HandlerResponse::Reply 供 middleware 存记忆。 |

---

## 6. telegram-bot

| 项目 | 说明 |
|------|------|
| **路径** | `telegram-bot/` |
| **描述** | 主应用：从环境加载配置、组装持久化/记忆/AI 等组件与 HandlerChain，使用 dbot-telegram 的 run_repl 启动 REPL。依赖 dbot-core、dbot-telegram、ai-client、ai-handlers、storage、memory、middleware、handler-chain 等；构造 OpenAILlmClient 与 TelegramBotAdapter 注入 SyncAIHandler。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口；再导出 dbot_telegram 的 TelegramMessageWrapper/TelegramUserWrapper，导出 BotConfig、run_bot、TelegramBot。 |
| `src/config.rs` | BotConfig：从环境变量加载 token、数据库、AI、记忆、embedding 等配置。 |
| `src/telegram_impl.rs` | 实现 dbot_core::Bot 的 TelegramBot 封装（send_message、reply_to、edit_message、send_message_and_return_id），内部委托 teloxide::Bot。 |
| `src/runner.rs` | run_bot、BotComponents；构造 OpenAILlmClient、TelegramBotAdapter，传入 SyncAIHandler；初始化存储/记忆/embedding、构建 HandlerChain，调用 dbot_telegram::run_repl 启动 REPL。 |

---

## 7. telegram-bot-ai

| 项目 | 说明 |
|------|------|
| **路径** | `telegram-bot-ai/` |
| **描述** | 简单 AI 机器人：依赖 ai-client；TelegramBotAI 内部持有 OpenAILlmClient，提供 get_ai_response/流式接口与 @ 提及解析；main 为独立 REPL 二进制（无 RAG）。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | TelegramBotAI：持有 OpenAILlmClient；get_ai_response、get_ai_response_with_messages、流式接口委托 llm_client；@mention 检测与 handle_message（Teloxide Bot + Message）。 |
| `src/main.rs` | 独立可执行：从 env 读 BOT_TOKEN/OPENAI_*，构造 OpenAILlmClient 传入 TelegramBotAI，启动 REPL，流式回复。 |

---

## 8. openai-client

| 项目 | 说明 |
|------|------|
| **路径** | `openai-client/` |
| **描述** | OpenAI 兼容 HTTP 客户端：Chat Completions 与流式完成、请求/响应结构。被 telegram-bot-ai、后续 ai-client 等调用。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | OpenAIClient、ChatCompletion 请求/响应类型、chat_completion 与 chat_completion_stream。 |
| `src/main.rs` | 可选的本地运行入口（如测试/演示）。 |

---

## 8. dbot-cli

| 项目 | 说明 |
|------|------|
| **路径** | `dbot-cli/` |
| **描述** | 命令行工具：对存储/记忆等做查询或管理，独立于 Bot 运行。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/main.rs` | CLI 入口：解析子命令、调用 storage 等。 |

---

## 10. telegram-bot-examples

| 项目 | 说明 |
|------|------|
| **路径** | `telegram-bot-examples/` |
| **描述** | 预留的示例占位 crate，后续可在此添加新的示例二进制。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 占位库，无示例二进制时保证 crate 可编译。 |

---

## 11. memory

| 项目 | 说明 |
|------|------|
| **路径** | `memory/` |
| **描述** | 对话记忆聚合：从 memory-core / memory-strategies 再导出类型与策略，提供 Context、ContextBuilder、estimate_tokens，供 RAG 构建上下文。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口；再导出 memory_core、memory_strategies 及 context 模块。 |
| `src/context/mod.rs` | Context 模块入口。 |
| `src/context/builder.rs` | ContextBuilder：按策略与 token 限制构建 Context。 |
| `src/context/types.rs` | Context 及相关类型。 |
| `src/context/utils.rs` | 上下文构建辅助。 |
| `src/migration.rs` | 记忆数据迁移相关（若存在）。 |

---

## 12. memory-core

| 项目 | 说明 |
|------|------|
| **路径** | `crates/memory-core/` |
| **描述** | 记忆核心类型与 trait：MemoryEntry、MemoryMetadata、MemoryRole、MemoryStore、StrategyResult。被 memory、memory-strategies、各存储实现使用。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口；再导出 types、store、strategy_result。 |
| `src/types.rs` | MemoryEntry、MemoryMetadata、MemoryRole 等。 |
| `src/store.rs` | MemoryStore trait（增删查等）。 |
| `src/strategy_result.rs` | StrategyResult：策略返回结果枚举。 |

---

## 13. memory-strategies

| 项目 | 说明 |
|------|------|
| **路径** | `crates/memory-strategies/` |
| **描述** | 上下文策略实现：RecentMessages、SemanticSearch、UserPreferences 等，实现 ContextStrategy，供 ContextBuilder 组合使用。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口；导出各策略与 ContextStrategy。 |
| `src/strategy.rs` | ContextStrategy trait 定义。 |
| `src/recent_messages.rs` | RecentMessagesStrategy。 |
| `src/semantic_search.rs` | SemanticSearchStrategy（依赖 EmbeddingService）。 |
| `src/user_preferences.rs` | UserPreferencesStrategy。 |
| `src/utils.rs` | 策略共用工具。 |

---

## 14. embedding

| 项目 | 说明 |
|------|------|
| **路径** | `crates/embedding/` |
| **描述** | 文本向量化抽象：EmbeddingService trait（embed、embed_batch）。被 memory-strategies、openai-embedding、bigmodel-embedding 等实现或使用。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | EmbeddingService trait 及文档。 |

---

## 15. openai-embedding

| 项目 | 说明 |
|------|------|
| **路径** | `crates/openai-embedding/` |
| **描述** | OpenAI 文本嵌入实现：实现 embedding::EmbeddingService，调用 OpenAI Embeddings API。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | OpenAI Embedding 实现。 |
| `src/openai_embedding_test.rs` | 嵌入相关测试（仅 test）。 |

---

## 16. bigmodel-embedding

| 项目 | 说明 |
|------|------|
| **路径** | `crates/bigmodel-embedding/` |
| **描述** | 智谱 BigModel 文本嵌入实现：实现 embedding::EmbeddingService。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | BigModel 嵌入实现。 |
| `src/bigmodel_embedding_test.rs` | 嵌入相关测试（仅 test）。 |

---

## 17. memory-inmemory

| 项目 | 说明 |
|------|------|
| **路径** | `crates/memory-inmemory/` |
| **描述** | 内存版 MemoryStore 实现：无持久化，用于测试或简单场景。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | InMemoryVectorStore 等实现。 |

---

## 18. memory-sqlite

| 项目 | 说明 |
|------|------|
| **路径** | `crates/memory-sqlite/` |
| **描述** | 基于 SQLite 的 MemoryStore 实现：向量/记忆落库。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | SQLite 存储实现与对外 API。 |

---

## 19. memory-lance

| 项目 | 说明 |
|------|------|
| **路径** | `crates/memory-lance/` |
| **描述** | 基于 Lance 的向量存储实现：MemoryStore，用于语义检索与 RAG。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口与公开 API。 |
| `src/config.rs` | Lance 配置（路径、维度等）。 |
| `src/store.rs` | Lance 存储实现。 |
| `src/distance_type.rs` | 距离类型枚举。 |
| `src/index_type.rs` | 索引类型枚举。 |

---

## 20. memory-loader

| 项目 | 说明 |
|------|------|
| **路径** | `crates/memory-loader/` |
| **描述** | 记忆数据加载与转换：从外部格式导入到 MemoryStore 的配置与转换逻辑。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口与对外 API。 |
| `src/config.rs` | 加载配置。 |
| `src/converter.rs` | 格式转换实现。 |
| `src/converter_test.rs` | 转换逻辑测试（仅 test）。 |

---

## 21. handler-chain

| 项目 | 说明 |
|------|------|
| **路径** | `crates/handler-chain/` |
| **描述** | 处理链：按顺序执行 Middleware before → Handler 列表 → Middleware after；Handler 返回 Continue/Stop/Reply 等控制流程。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | HandlerChain 类型、add_middleware/add_handler、handle 逻辑及链上测试。 |

---

## 22. middleware

| 项目 | 说明 |
|------|------|
| **路径** | `crates/middleware/` |
| **描述** | 中间件实现：LoggingMiddleware、AuthMiddleware、PersistenceMiddleware、MemoryMiddleware 等，实现 dbot_core::Middleware。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | 入口；导出 LoggingMiddleware、AuthMiddleware、MemoryMiddleware、PersistenceMiddleware。 |
| `src/middleware.rs` | LoggingMiddleware、AuthMiddleware 等通用中间件。 |
| `src/persistence_middleware.rs` | PersistenceMiddleware：消息持久化前后钩子。 |
| `src/memory_middleware.rs` | MemoryMiddleware：记忆写入/读取相关前后钩子。 |
| `src/test/mod.rs` | 测试模块入口。 |
| `src/test/memory_middleware_test.rs` | MemoryMiddleware 测试。 |
| `src/test/persistence_middleware_test.rs` | PersistenceMiddleware 测试。 |

---

## 23. prompt

| 项目 | 说明 |
|------|------|
| **路径** | `crates/prompt/` |
| **描述** | 提示与消息格式：MessageRole、ChatMessage、format_for_model、format_for_model_as_messages 等，供 LLM 调用方组消息。 |

### 文件与描述

| 文件 | 描述 |
|------|------|
| `src/lib.rs` | MessageRole、ChatMessage、format_for_model、format_for_model_as_messages、parse_message_line 等。 |

---

## 索引表（按职责）

| 职责 | Crate |
|------|-------|
| 核心类型与抽象 | dbot-core |
| Telegram 框架层 | dbot-telegram |
| 消息持久化 | storage |
| 链式执行 | handler-chain |
| 中间件实现 | middleware |
| AI 处理（RAG/提及） | ai-handlers |
| 主应用与 Telegram 组装 | telegram-bot |
| 简单 AI 机器人 | telegram-bot-ai |
| LLM HTTP 客户端 | openai-client |
| 提示与消息格式 | prompt |
| 记忆聚合与上下文 | memory |
| 记忆核心类型 | memory-core |
| 记忆策略 | memory-strategies |
| 向量化抽象 | embedding |
| 向量化实现 | openai-embedding, bigmodel-embedding |
| 记忆存储实现 | memory-inmemory, memory-sqlite, memory-lance |
| 记忆加载 | memory-loader |
| CLI 工具 | dbot-cli |
| 示例占位 | telegram-bot-examples |
