# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Telegram 编辑请求频率控制**
  - **telegram-bot**：`BotConfig` 新增 `telegram_edit_interval_secs`（环境变量 `TELEGRAM_EDIT_INTERVAL_SECS`，默认 5 秒）；流式回复时两次编辑同一条消息的最小间隔可配置，避免触发 Telegram 限流。
  - **ai-handlers**：`SyncAIHandler` 新增 `edit_interval_secs` 参数；在流式编辑循环中按间隔节流：首次编辑立即执行，后续编辑等待距上次编辑满 `edit_interval_secs` 秒后再发请求。
  - **配置**：`.env.example` 增加 `TELEGRAM_EDIT_INTERVAL_SECS` 说明；config 单测覆盖默认值与自定义值。
- **框架层 dbot-telegram（P0）**
  - 新增 crate `crates/dbot-telegram`：Telegram 接入与消息链运行，最小配置（token、api_url、log_file）。
  - 提供：`TelegramMessageWrapper`、`TelegramUserWrapper`（teloxide→core）、`TelegramBotAdapter`（实现 dbot_core::Bot）、`TelegramConfig`、`run_repl(bot, handler_chain, bot_username)`。
  - **telegram-bot** 改为依赖 dbot-telegram：移除本地 `adapters.rs`，从 dbot_telegram 再导出 Wrapper；`run_bot` 内调用 `dbot_telegram::run_repl` 启动 REPL，行为与重构前一致。
  - 文档：CRATES.md 新增 dbot-telegram 条目并更新 telegram-bot 描述；README 项目结构可引用 dbot-telegram。
- **AI 抽象层 ai-client + Bot 扩展（P1）**
  - 新增 crate `crates/ai-client`：仅依赖 openai-client、prompt；定义 `LlmClient` trait（get_ai_response_with_messages、get_ai_response_stream_with_messages）、`StreamChunk`、`OpenAILlmClient` 实现（从 telegram-bot-ai 迁出 LLM 调用逻辑）。
  - **dbot-core**：Bot trait 新增 `edit_message(chat, message_id, text)`、`send_message_and_return_id(chat, text)`，用于流式回复「先发一条再编辑」。
  - **dbot-telegram**：TelegramBotAdapter 实现上述两方法，委托 teloxide 的 edit_message_text / send_message。
  - **telegram-bot**：TelegramBot（telegram_impl）同样实现 edit_message、send_message_and_return_id。
  - 文档：CRATES.md 新增 ai-client 条目。
- **应用与 Handler 重构（P2）**
  - **ai-handlers**：SyncAIHandler 改为持有 `Arc<OpenAILlmClient>`、`Arc<dyn dbot_core::Bot>`，不再依赖 telegram-bot-ai、teloxide；发消息统一用 `Bot::send_message`，流式用 `Bot::send_message_and_return_id` + `Bot::edit_message`；构造处接收 llm_client + bot。
  - **telegram-bot**：移除 telegram-bot-ai 依赖，增加 ai-client；runner 内构造 `OpenAILlmClient`（用 config 的 AI 字段）与 `TelegramBotAdapter`，将 `Arc<OpenAILlmClient>`、`Arc<dyn Bot>` 传入 SyncAIHandler::new；主应用 RAG + 流式/非流式行为不变。
  - **telegram-bot-ai**：改为依赖 ai-client（移除对 openai-client、prompt 的直接依赖）；TelegramBotAI 内部持有 `OpenAILlmClient`，get_ai_response / stream 等方法委托 llm_client；main 仍为独立 REPL 二进制。
  - 测试：ai-handlers 单元测试使用 MockBot + anyhow::Result（EmbeddingService mock），避免与 dbot_core::Result 冲突；全 workspace 构建与测试通过。
- **回复消息内容作为 AI 上下文**
  - **dbot-core**：`Message` 新增字段 `reply_to_message_content: Option<String>`，存储被回复消息的文本内容。
  - **telegram-bot adapters**：`TelegramMessageWrapper::to_core()` 新增 `get_reply_to_message_content()` 方法，从 Telegram 的 `reply_to_message.text` 提取被回复消息内容。
  - **ai-handlers**：`SyncAIHandler::build_messages_for_ai()` 新增参数 `reply_to_content: Option<&str>`；当用户回复机器人消息时，被回复的内容作为 assistant 消息插入到最后一条 user 消息前，让 AI 了解对话上下文。
  - **单元测试**：`sync_ai_handler_test.rs` 新增 `test_reply_to_bot_with_content_returns_question`、`test_reply_to_bot_content_is_preserved`、`test_reply_to_non_bot_with_content` 等测试。
  - **其他**：所有测试文件中 `Message` 构造已同步更新。


- **RecentMessages 使用 SQLite、语义搜索不受影响（双 store）**
  - **memory-strategies**：`ContextStrategy` 新增 `store_kind() -> StoreKind`（Primary / Recent）；`RecentMessagesStrategy`、`UserPreferencesStrategy` 为 Recent，`SemanticSearchStrategy` 为 Primary；导出 `StoreKind`。
  - **memory**：`ContextBuilder` 新增 `recent_store: Option<Arc<dyn MemoryStore>>` 与 `with_recent_store(recent_store)`；`build()` 时按策略的 `store_kind()` 选择主 store 或 recent_store 传入。
  - **middleware**：`MemoryConfig` 新增 `recent_store: Option<Arc<dyn MemoryStore>>`；`with_store_and_embedding(store, embedding_service, recent_store)` 增加第三参数；before/after 写入主 store 后，若 recent_store 存在且与主 store 不同则同时写入 recent_store。
  - **ai-handlers**：`SyncAIHandler::new` 增加 `recent_store: Option<Arc<dyn MemoryStore>>`；`build_memory_context` 中当 `recent_store` 为 Some 时对 `ContextBuilder` 调用 `with_recent_store`。
  - **telegram-bot**：`BotConfig` 新增 `memory_recent_use_sqlite: bool`（环境变量 `MEMORY_RECENT_USE_SQLITE`，1/true/yes 为 true）；runner 在 `memory_recent_use_sqlite` 时创建 SQLite 存储并作为 recent_store 传入；`BotComponents` 新增 `recent_store`；`MemoryMiddleware` 与 `SyncAIHandler` 使用该 recent_store。
  - **文档**：`.env.example`、`.env.test.example` 增加 `MEMORY_RECENT_USE_SQLITE` 与相关说明；config 单测 `test_load_config_memory_recent_use_sqlite`。

### Documentation
- **文档整理与简化**
  - 新增 `docs/README.md` 作为文档索引（根目录、RAG、CLI、测试、重构等入口）。
  - 新增 `docs/refactoring/README.md` 作为重构文档索引。
  - 根目录 `MEMORY.md` 简化为概述与链接，详细内容指向 `docs/rag/` 与 `docs/rag/memory/`。
  - `docs/RAG_SOLUTION.md` 简化为跳转到 `docs/rag/`。
  - `docs/rag/README.md` 移除末尾重复的「相关文档」小节。
  - 向量搜索准确度评审结论并入 `docs/rag/vector-search-accuracy-plan.md`；`vector-search-accuracy-plan-review.md` 改为简短重定向。
  - 根目录 `README.md` 增加「文档索引」链接。
- **全量 .md 简化（第二轮）**
  - docs/：api-endpoints、LOGGING、db_query_result、AI_MENTION_REPLY、PERSISTENCE_CHECK、TEST_COVERAGE、rust-telegram-bot-plan、dbot-cli-code-review、TELEGRAM_BOT_TEST_PLAN、RUST_TELEGRAM_BOT_GUIDE 等缩短为要点与表格，移除冗长代码与重复段落。
  - docs/rag/：architecture、data-flow、implementation、configuration、usage、testing、context-sending-scheme、context-retrieval-before-reply、cost、future、technical-selection、references、context_builder_design、LANCE_USAGE、LANCE_API_RESEARCH、DEVELOPMENT_PLAN 等简化为索引与核心结论。
  - docs/rag/memory/：README、types、storage、embeddings、usage、testing、vector-search-accuracy 等简化为表格与链接。
  - docs/refactoring/：DEVELOPMENT_PLAN、crate-split-bot-ai-plan、telegram-bot-framework-design、handler-chain-extraction 等简化为目标与阶段摘要。
  - 根目录与 crate：SETUP.md、dbot-core/ARCHITECTURE.md、dbot-cli/README（保持）、crates/memory-loader/README 等简化为步骤与配置要点。
- **策略与 SQLite：最近消息来自 SQLite，仍执行向量检索**
  - **memory-strategies**：crate 文档说明 RecentMessagesStrategy 从 store（如 SQLite）取最近消息，SemanticSearchStrategy 仍做向量检索；使用 SQLite 时二者均由同一 SQLite 存储提供。
  - **docs/rag/memory/storage.md**：SQLiteVectorStore 说明补充：支持 `search_by_*`（最近消息）与 `semantic_search`（向量检索）。

### Fixed
- **用户仅 @ 提及机器人、无内容时机器人无响应**
  - **ai-handlers**：`SyncAIHandler::get_question()` 在检测到 @ 提及但提取内容为空时，改为返回默认提示 `DEFAULT_EMPTY_MENTION_QUESTION`（让 AI 简短打招呼并邀请用户提问），不再返回 `None`，从而触发回复。
  - 新增关联常量 `SyncAIHandler::DEFAULT_EMPTY_MENTION_QUESTION`；单元测试 `test_get_question_mention_only_returns_none` 更名为 `test_get_question_mention_only_returns_default` 并断言返回该默认值。
- **最近消息上下文出现大量 "User: " / "Assistant: " 无内容行**
  - **原因**：Lance 查询返回的 RecordBatch 列序可能与代码假设不一致，或历史存在 content 为空的条目。
  - **memory-lance**：`batch_to_entry` 改为按**列名**（schema.index_of）读取列，不依赖列序，避免误读 content。
  - **memory-strategies**：`RecentMessagesStrategy` 在 `search_by_conversation` / `search_by_user` 后 `retain(|e| !e.content.is_empty())`，过滤掉 content 为空的条目，避免空行进入上下文。

### Changed
- **Bot 系统提示改为很嗲的台湾妹人设，Burt/Jason 为厌恶对象**
  - **telegram-bot-ai**：`DEFAULT_SYSTEM_CONTENT` 改为讲话很嗲的台湾女生，语气软萌、台湾腔；若对话中出现 Burt 或 Jason 则用嫌弃/不爽语气回应；仍要求中文、纯文本、无 Markdown，适合 Telegram。
- **系统提示词支持 .env 配置**
  - **telegram-bot**：`BotConfig` 新增 `ai_system_prompt: Option<String>`，从环境变量 `AI_SYSTEM_PROMPT` 读取；未设置时使用 telegram-bot-ai 内置默认。
  - **telegram-bot-ai**：`TelegramBotAI` 新增 `system_prompt: Option<String>` 及 `with_system_prompt` / `with_system_prompt_opt`；请求时优先使用配置的提示词。
  - **文档**：`telegram-bot/.env.example`、`.env.test.example` 增加 `AI_SYSTEM_PROMPT` 说明；config 单测增加 `test_load_config_ai_system_prompt`。
- **回复触发 AI 时仅认「回复机器人的消息」，@mention 保留**
  - **行为**：① 用户「回复机器人的某条消息」时触发 AI；② 用户 @ 提及机器人且问题非空时也触发 AI。若用户回复的是其他人的消息（非机器人），不因回复触发。
  - **dbot-core**：`Message` 新增字段 `reply_to_message_from_bot: bool`，表示被回复的那条消息是否由机器人发送。
  - **telegram-bot adapters**：将 Telegram 的 `reply_to_message.from.is_bot` 映射为 `reply_to_message_from_bot`。
  - **ai-handlers**：回复路径仅在 `reply_to_message_id.is_some() && reply_to_message_from_bot` 时触发；@mention 路径保留（优先回复、其次 @mention）。
  - **测试**：同步更新各 crate 中 `Message` 构造与相关单测。

### Added
- **CLI 查询向量数据库最近 N 条记录**
  - **dbot-cli**：新增子命令 `list-vectors`，按时间倒序输出 LanceDB 中最近 N 条记录（默认 100）；支持 `--limit`、`--lance-db-path`；环境变量 `LANCE_DB_PATH`、`LANCE_EMBEDDING_DIM`。
  - **memory-lance**：`LanceVectorStore` 新增非 trait 方法 `list_recent(limit)`，全表扫描后在内存按 `timestamp` 降序取前 limit 条。
  - **文档**：`docs/dbot-cli-vector-query-plan.md` 方案；`dbot-cli/README.md` 增加 list-vectors 用法与环境变量；`memory-lance` 集成测试增加 `test_lance_list_recent_returns_ordered_by_timestamp_desc`。
- **向量搜索准确度优化阶段 1：配置接入（Top-K 与相关项）**
  - **BotConfig**：新增 `memory_recent_limit`、`memory_relevant_top_k`，从环境变量 `MEMORY_RECENT_LIMIT`、`MEMORY_RELEVANT_TOP_K` 读取，默认值分别为 10、5。
  - **runner**：初始化 SyncAIHandler 时传入上述配置，用于构造 ContextBuilder 的 RecentMessagesStrategy / SemanticSearchStrategy。
  - **SyncAIHandler**：使用配置的 `memory_recent_limit`、`memory_relevant_top_k` 构建 ContextBuilder，不再写死 10/5。
  - **文档**：`.env.example` 增加 MEMORY_RECENT_LIMIT、MEMORY_RELEVANT_TOP_K 注释；`docs/rag/configuration.md` 增加「Telegram Bot 实现的环境变量」表及推荐范围（recent 5–20，top_k 3–10）。
  - **单元测试**：`telegram-bot/src/config.rs` 新增 `test_load_config_memory_recent_limit_and_top_k`，覆盖默认值与显式设置。
- **向量搜索准确度优化开发计划**：新增 `docs/rag/vector-search-accuracy-plan.md`，以表格形式列出配置接入（MEMORY_RELEVANT_TOP_K / MEMORY_RECENT_LIMIT）、相似度阈值过滤、Lance 检索精度可选优化及文档与 CHANGELOG 等任务；`docs/rag/README.md` 增加该计划入口链接。
- **向量搜索准确度优化阶段 2：相似度阈值过滤与可观测性**
  - **设计决策**：`MemoryStore::semantic_search` 扩展为返回 `Vec<(f32, MemoryEntry)>`（相似度分数 + 条目），决策与理由记录在 `docs/rag/memory/vector-search-accuracy.md`。
  - **memory-core**：`semantic_search` 返回类型改为 `Result<Vec<(f32, MemoryEntry)>, anyhow::Error>`。
  - **memory-lance**：从 Lance 结果 batch 读取 `_distance` 列并转换为相似度分数后返回；无 `_distance` 时使用 1.0。
  - **memory-sqlite / memory-inmemory**：语义检索内部已有相似度计算，改为返回 `(score, entry)` 列表。
  - **SemanticSearchStrategy**：新增 `min_score: f32` 参数（0.0 表示不过滤）；只保留 `score >= min_score` 的条目；打日志「向量检索 分数分布」（min/mean/max、条数）；全部被阈值过滤时打 warning。
  - **BotConfig**：新增 `memory_semantic_min_score`，从环境变量 `MEMORY_SEMANTIC_MIN_SCORE` 读取，默认 0.0。
  - **文档**：`docs/rag/memory/vector-search-accuracy.md` 含配置项、成本与准确度权衡表、异常与降级说明；`docs/rag/configuration.md` 与 `.env.example` 增加 MEMORY_SEMANTIC_MIN_SCORE。
  - **单元测试**：`memory-strategies/tests/semantic_search_test.rs` 增加 `test_semantic_search_min_score_filters_low_scores`、`test_semantic_search_min_score_zero_keeps_all`；config 测试增加对 `memory_semantic_min_score` 默认值与显式设置的断言。
- **向量搜索准确度优化阶段 3：Lance 检索精度可选与 fetch_limit 可配置**
  - **调研**：`docs/rag/LANCE_API_RESEARCH.md` 增加「Accuracy vs speed」小节，记录 Lance Rust 0.23 的 `bypass_vector_index()`（精确/暴力搜索）、`refine_factor`（IVF-PQ 精排）、`nprobes`（IVF 分区数）。
  - **memory-lance LanceConfig**：新增 `use_exact_search`（默认 false）、`refine_factor`（Option<u32>）、`nprobes`（Option<usize>）、`semantic_fetch_multiplier`（默认 10）；默认值不改变现有调用方行为。
  - **LanceVectorStore::semantic_search**：按配置在构建查询时链式调用 `bypass_vector_index()`、`refine_factor(rf)`、`nprobes(np)`；过滤时 `fetch_limit = limit × semantic_fetch_multiplier`（至少 50）。
  - **文档**：`docs/rag/LANCE_USAGE.md` 增加「准确度与速度权衡」表与 LanceConfig 示例；`docs/rag/memory/vector-search-accuracy.md` 增加「Lance 检索参数」表；`docs/rag/vector-search-accuracy-plan.md` 阶段 3 任务标记为已完成。
- **向量搜索准确度优化阶段 4：文档与 CHANGELOG 收尾**
  - **docs/rag/README.md**：已包含「向量搜索准确度优化计划」「向量搜索准确度」子页及「配置」入口链接（阶段 2 已加）。
  - **docs/rag/memory/vector-search-accuracy.md**：补全「距离度量与索引选择」「Embedding 模型建议」小节；含 Top-K、阈值、Lance 检索参数、三档推荐、异常与降级。
  - **CHANGELOGS.md**：阶段 1–3 相关变更已按 [Unreleased] 记录；本阶段标记计划 4.1–4.3 已完成。
- **向量搜索准确度优化任务 4.4：语义检索回归集**
  - **memory-lance**：新增集成测试 `test_semantic_search_regression_golden_cases`，3 条黄金用例（one-hot 查询→期望命中 "entry A/B/C"），可复现、无外部 API，便于 CI 稳定。
  - **docs/rag/memory/vector-search-accuracy.md**：新增「语义检索回归集（黄金用例）」小节，约定 fixture、用例表与验证命令。
- **写入记忆时算 embedding**：MemoryMiddleware 保存用户消息与 AI 回复时若配置了 embedding_service，则先对 content 做 embed，再写入 entry.embedding，使新消息参与语义检索。
  - **middleware**：MemoryConfig 新增 `embedding_service: Option<Arc<dyn EmbeddingService>>`；新增 `MemoryMiddleware::with_store_and_embedding(store, embedding_service)`；before()/after() 中若 embedding_service 为 Some 则调用 `embed(&content).await` 并设置 `entry.embedding`，失败时仍保存但不带 embedding。
  - **telegram-bot runner**：BotComponents 新增 `embedding_service`；创建 MemoryMiddleware 改为 `with_store_and_embedding(components.memory_store, components.embedding_service)`。
- **语义检索按用户/会话限定**：semantic_search 支持按 user_id、conversation_id 过滤，只返回当前用户/会话内的相似结果。
  - **memory-core**：`MemoryStore::semantic_search` 增加参数 `user_id: Option<&str>`, `conversation_id: Option<&str>`。
  - **memory-lance**：先取更多候选（limit*10 或 50），再按 user_id/conversation_id 过滤后取前 limit 条。
  - **memory-sqlite / memory-inmemory**：在计算相似度前按 user_id/conversation_id 过滤。
  - **memory-strategies (SemanticSearchStrategy)**：调用 store.semantic_search 时传入 `user_id.as_deref()`, `conversation_id.as_deref()`。
  - **telegram-bot tests (MockMemoryStore)**：semantic_search 实现过滤逻辑；调用处补全新参数。

### Fixed
- **最近消息查询顺序**：RecentMessagesStrategy 与 Lance 存储的「最近消息」顺序修正
  - **memory-lance**：`search_by_conversation`、`search_by_user` 返回前按 `metadata.timestamp` 升序排序，保证顺序确定。
  - **memory-strategies (RecentMessagesStrategy)**：拿到 store 结果后先按 `metadata.timestamp` 升序排序，再取**最后** `limit` 条（即「最近 N 条」按时间从旧到新），再格式化为消息。这样无论底层是 Lance（无排序）、SQLite（DESC）还是 InMemory，最近消息均为「最近 N 条、时间正序」，提交给 AI 的 Conversation (recent) 顺序正确。

### Removed
- **Context 相关冗余代码与文档同步**
  - **ai-handlers**：删除未在流程中使用的 `SyncAIHandler::format_question_with_context`；当前流程使用 `build_messages_for_ai` → `Context::to_messages` + `get_ai_response_with_messages` / `get_ai_response_stream_with_messages`。
  - **memory**：删除仅被单测使用的 `Context::conversation_history()`；生产代码无调用方。
  - **测试**：`sync_ai_handler_test.rs` 移除对 `format_question_with_context` 的测试；`context_test.rs` 移除 `test_conversation_history_returns_recent_then_semantic`。
  - **文档**：`docs/rag/context-sending-scheme.md` 与 `docs/rag/context-retrieval-before-reply.md` 更新为当前实现（`to_messages` + `get_ai_response_with_messages`），不再描述已废弃的「单条 user 消息 + format_question_with_context」流程。

### Added
- **Context 返回带类型的 chat messages（system / user / assistant）**
  - **prompt crate**：新增 `parse_message_line(line)`，解析 "User: ..." / "Assistant: ..." / "System: ..." 为对应角色的 `ChatMessage`；新增 `format_for_model_as_messages_with_roles(...)`，将 recent 对话行解析为多条带正确角色的消息，再追加可选的 preferences+semantic 块与当前问题。
  - **memory Context**：`to_messages(include_system, current_question)` 改为调用 `prompt::format_for_model_as_messages_with_roles`，返回的 `Vec<ChatMessage>` 中 recent 为多条 User/Assistant/System，与 OpenAI messages 数组一一对应。
  - **单元测试**：`prompt/tests/format_for_model_test.rs` 增加 `parse_message_line` 与 `format_for_model_as_messages_with_roles` 测试；`memory/context_test.rs` 增加 `test_to_messages_returns_chat_messages_with_different_roles`。
- **prompt crate**：将 AI prompt 格式化逻辑独立为 crate
  - 新增 `crates/prompt`：提供 `format_for_model(include_system, system_message, user_preferences, recent_messages, semantic_messages)` 及常量 `SECTION_RECENT` / `SECTION_SEMANTIC`；无依赖，可被 memory 或其他上下文来源复用。
  - **memory**：`Context::format_for_model()` 委托给 `prompt::format_for_model()`，格式规则集中在 prompt crate。
- **Context: 区分最近对话与语义检索，并将最近对话作为 AI 对话记录**
  - **memory-core**: 新增 `MessageCategory` 枚举（`Recent` / `Semantic`），`StrategyResult::Messages` 改为带 `category` 和 `messages`，供 ContextBuilder 按来源分类。
  - **memory-strategies**: `RecentMessagesStrategy` 返回 `Messages { category: Recent, messages }`，`SemanticSearchStrategy` 返回 `Messages { category: Semantic, messages }`。
  - **memory Context**: `conversation_history` 拆分为 `recent_messages`（主对话记录）与 `semantic_messages`（相关参考）；新增 `conversation_history()` 返回合并列表、`is_empty()` 判断是否无消息。
  - **format_for_model()**: 输出分段标题「Conversation (recent):」与「Relevant reference (semantic):」，模型可区分主对话与检索参考。
  - 调用方与测试已更新：middleware、ai-handlers 使用 `context.is_empty()`；集成测试使用 `context.recent_messages` / `context.semantic_messages`。
- **memory-lance**: Lance + SemanticSearchStrategy 完整策略集成测试（真实词向量与存储验证）
  - 新增 `tests/lance_semantic_strategy_integration_test.rs`：使用临时 Lance 库、可复现的 1536 维 one-hot 向量和按查询返回向量的 Mock Embedding，验证 `SemanticSearchStrategy.build_context` 对查询「猫」返回「关于猫」的最近邻消息；并验证无查询时返回 Empty。
  - 测试依赖：`embedding`、`async-trait` 作为 dev-dependencies，便于在测试中实现 `EmbeddingService`。
  - 文档：`docs/rag/LANCE_USAGE.md` 增加「完整策略：Lance + SemanticSearchStrategy」与验证方式；`docs/rag/memory/storage.md` 将 LanceVectorStore 列为已实现并注明集成测试。
- **消息处理全流程 step 日志**：收到消息后的每个步骤均打 info 日志，便于排查与追踪
  - **runner**：收到消息打 "Received message"；spawn 处理任务时打 "step: processing message (handler chain started)"。
  - **handler_chain**：开始打 "step: handler_chain started"；每个 middleware before 前后打 "step: middleware before" / "step: middleware before done"（含 middleware 类型名）；每个 handler 前后打 "step: handler processing" / "step: handler done"（含 response_type、reply_len，不打印完整 Reply 内容）；每个 middleware after 前后打 "step: middleware after" / "step: middleware after done"；结束打 "step: handler_chain finished"。
  - **PersistenceMiddleware**：before 打 "step: PersistenceMiddleware before, saving message"、保存后打 "step: PersistenceMiddleware before done, message saved"；after 打 "step: PersistenceMiddleware after"。
  - **MemoryMiddleware**：before 打 "step: MemoryMiddleware before, saving user message to memory"、保存后打 "step: MemoryMiddleware before done, user message saved to memory"（或 save_user_messages=false 时打 skip）；after 打 "step: MemoryMiddleware after"、若保存 AI 回复打 "step: MemoryMiddleware after done, AI reply saved to memory"（无 Reply 或 save_ai_responses=false 时打相应 skip）。
  - **SyncAIHandler**：进入打 "step: SyncAIHandler handle start"；非 AI 查询（无 reply-to、无 @mention）打 "step: SyncAIHandler not AI query, skip" 并返回；AI 查询仍保留原有 "Replying to bot message" / "Bot mentioned, processing" 及 "Submitting to AI" 等日志。
- **词向量 (embedding/vector) 处理日志**：处理词向量时在各层打印 step 日志，便于排查 RAG 与向量检索
  - **SemanticSearchStrategy**：生成查询向量前 "step: 词向量 生成查询向量 (embedding)"（含 query_preview、query_len）；生成完成后 "step: 词向量 查询向量生成完成"（dimension）；向量检索前 "step: 词向量 向量检索 (semantic_search)"（dimension、limit）；检索完成后 "step: 词向量 向量检索完成"（entry_count）。
  - **OpenAI embedding**：单条 "step: 词向量 OpenAI embed 请求"（model、text_preview、text_len）、"step: 词向量 OpenAI embed 完成"（dimension）；批量 "step: 词向量 OpenAI embed_batch 请求/完成"（batch_size、count、dimension）。
  - **BigModel embedding**：单条 "step: 词向量 BigModel embed 请求/完成"；批量 "step: 词向量 BigModel embed_batch 请求/完成"（依赖新增 tracing）。
  - **向量存储 (Lance/SQLite/InMemory)**：add 时若 entry 带 embedding 打 "step: 词向量 [Lance|SQLite|InMemory] 写入向量"（id、dimension）；semantic_search 时打 "step: 词向量 [Store] 向量检索"（dimension、limit）、"step: 词向量 [Store] 向量检索完成"（count）。
- **Context 详细日志：最近消息、语义检索、用户偏好**
  - **memory (ContextBuilder)**：构建完成后调用 `log_context_detail`，按块打印：`context_detail: 最近消息`（条数 + 每条 index、content_preview 500 字）；`context_detail: 语义检索`（条数 + 每条 index、content_preview）；`context_detail: 用户偏好`（preferences_preview 或「无」）。`apply_strategy_result` 在合并每条策略结果时，对 Messages 逐条打 `strategy message`（label 为 最近消息/语义检索），对 Preferences 打 `Strategy returned user preferences (用户偏好)`，预览长度统一为 `CONTEXT_LOG_PREVIEW_LEN`（500）。
  - **memory (utils)**：新增常量 `CONTEXT_LOG_PREVIEW_LEN = 500`，供上下文详细日志截断用。
  - **memory-strategies**：策略层日志文案增加中文标签：RecentMessagesStrategy 为「最近消息」；SemanticSearchStrategy 为「语义检索」；UserPreferencesStrategy 为「用户偏好 extracted」。
- **RAG / context build**: Detailed logging for strategies and AI submission
  - **ContextStrategy**: Added `name(&self) -> &str` to trait; each strategy (RecentMessages, SemanticSearch, UserPreferences) returns a constant name for logging.
  - **memory-strategies**: Each strategy logs query results in detail: `RecentMessagesStrategy` and `SemanticSearchStrategy` log `entry_count`, `message_count`, and per-message `content_preview` (truncated to 400 chars). `UserPreferencesStrategy` logs `entry_count` and `preferences_preview` when preferences are found. On store/embedding failure, strategies log `error` and (for semantic search) `error_debug` and `query_preview`.
  - **memory (ContextBuilder)**: Executing each strategy logs `strategy_name` and `strategy_index`; for Messages result logs summary (`message_count`, `total_content_len`) and per-message with label 最近消息/语义检索 and `content_preview` (500 chars); for Preferences logs `preferences_preview` (用户偏好). After build, `log_context_detail` logs 最近消息/语义检索/用户偏好 blocks. On strategy failure, logs `strategy_name`, `error`, and full error chain (`Caused by`).
  - **ai-handlers (SyncAIHandler)**: Before calling the AI, logs `message_count`, `question` with message "Submitting to AI (streaming)" or "Submitting to AI (non-streaming)"; then logs **提交给 AI 的消息列表** via `log_messages_submitted_to_ai`: for each message logs `index`, `role` (System/User/Assistant), `content_preview` (truncated to 500 chars). On AI response failure (normal or stream), logs full error chain.
  - **memory-strategies/utils**: Added `truncate_for_log(s, max_len)` and `MAX_LOG_CONTENT_LEN` for safe content preview in logs.
- **openai-client**: Request logging and masked token
  - Log each chat completion request: `model`, `message_count`, and masked `api_key` (first 7 + `***` + last 4 chars; keys ≤11 chars log as `***`).
  - **提交的 JSON**：在发送前将请求体序列化为 JSON 并打印：`OpenAI chat_completion 提交的 JSON` / `OpenAI chat_completion_stream 提交的 JSON`，字段 `request_json` 为 `serde_json::to_string_pretty(&request)` 的完整请求体（含 model、messages 等）。
  - Log response token usage: `prompt_tokens`, `completion_tokens`, `total_tokens` for non-stream and stream (when API returns usage).
  - New public `mask_token(token)` for safe logging; optional `api_key_for_logging` stored in client when created via `new()` / `with_base_url()` (not set for `with_client()`).
  - Unit tests in `openai-client/tests/mask_token_test.rs` for `mask_token`.
  - Dependency: `tracing`.

### Changed
- **telegram-bot runner**: 移除外层循环，单次运行 repl
  - 不再使用 `loop` 包裹 `teloxide::repl`，不再在 long polling 退出后自动重连（无 `RECONNECT_DELAY_SECS`、无 `tokio::select!` 与 Ctrl+C 监听）；`repl` 返回后进程直接退出。
- **telegram-bot runner**: 每条消息在独立任务中处理，不阻塞轮询
  - 在 repl 闭包内用 `tokio::spawn` 执行 `handler_chain.handle`，收到消息后立即返回 `Ok(())`，实际处理在后台任务中完成，确保长时间处理（如 AI 调用）不会阻塞收下一条消息。

### Fixed
- **telegram-bot integration test**: Telegram API 改为 mock，不再访问真实 API
  - `test_ai_reply_complete_flow` 此前使用假 `BOT_TOKEN` 调用真实 Telegram，导致 "Invalid bot token"。
  - 新增可选配置 `BotConfig::telegram_api_url`（环境变量 `TELEGRAM_API_URL` 或 `TELOXIDE_API_URL`），runner 中若设置则对 `Bot` 调用 `set_api_url` 指向该 URL。
  - 集成测试中启动 `mockito::Server::new_async()`，注册 getMe / sendMessage mock（路径 `/bot<token>/getMe`、`/bot<token>/sendMessage`），并设置 `TELEGRAM_API_URL`，使 Telegram 请求发往本地 mock，无需真实 token。
  - 测试结束移除 `TELEGRAM_API_URL`，避免影响其他测试。

### Added
- **ai-handlers**: SyncAIHandler unit tests in dedicated directory (same level as `src`)
  - Added `ai-handlers/tests/sync_ai_handler_test.rs` (integration tests; `tests/` and `src/` are siblings).
  - Tests cover: `is_bot_mentioned`, `extract_question`, `get_question`, `format_question_with_context` (reply-to content, @mention extraction, empty context vs with context).
  - Uses in-memory SQLite, `MockEmbeddingService`, and dummy Bot/TelegramBotAI; no Telegram or OpenAI calls.
  - Exposed `is_bot_mentioned`, `extract_question`, `get_question`, `format_question_with_context` as `pub` with doc comments for tests in `tests/`; removed `src/tests/` and `#[cfg(test)] mod tests` from lib.rs.
- middleware: unit tests split into independent file
  - Added `crates/middleware/src/memory_middleware_test.rs` with all MemoryMiddleware unit tests (MemoryConfig default, creation, message_to_memory_entry, before/after saving, build_context); tests use InMemoryVectorStore and do not call external services
  - Removed inline `#[cfg(test)] mod tests` from `memory_middleware.rs`
  - Exposed `config`, `message_to_memory_entry`, `reply_to_memory_entry`, and `build_context` as `pub(crate)` for the test module; registered `#[cfg(test)] mod memory_middleware_test` in lib.rs
- SyncAIHandler: AI runs synchronously in the handler chain, returns Reply(text) for middleware to save (Option 1)
  - **ai-handlers**: new `SyncAIHandler` implementing `Handler`; on AI query (reply-to or @mention) builds context, calls AI (normal or streaming), sends reply to Telegram, returns `HandlerResponse::Reply(response_text)` so MemoryMiddleware saves it in `after()`. New module `sync_ai_handler.rs`.
  - **telegram-bot runner**: removed channel and async AI task; `BotComponents` now has `sync_ai_handler: Arc<SyncAIHandler>` instead of `query_sender` and `ai_query_handler`. Chain uses `sync_ai_handler`; user message saved in middleware `before()`, AI reply saved in middleware `after()` when handler returns `Reply(text)`. Removed `start_ai_handler()` and its call from `run_bot`.
  - **telegram-bot tests**: `test_ai_reply_complete_flow` no longer calls `start_ai_handler()` or polls; `handle_core_message` runs the full chain synchronously.
  - **AIQueryHandler** (async channel pipeline) was later removed; runner uses only **SyncAIHandler**.
- HandlerResponse::Reply(String) so middleware can receive reply text in after()
  - dbot-core: added variant `Reply(String)` to `HandlerResponse`; handlers that produce a reply can return it for middleware (e.g. memory) to use
  - handler-chain: treats `Reply(_)` like `Stop` (break chain, pass response to after())
  - memory_middleware: in after(), when response is `Reply(text)` and save_ai_responses is true, saves text as Assistant to MemoryStore via new `reply_to_memory_entry(message, text)`; removed TODO
  - ai-handlers: removed duplicate user-message save in handle_query (user message is already saved by MemoryMiddleware in before()); AI reply still saved in handler (current flow produces reply asynchronously, so after() does not see Reply yet)
- memory/context: split unit tests into separate files by type
  - **context/estimate_tokens_test.rs**: tests for `estimate_tokens` (token estimation)
  - **context/context_builder_test.rs**: tests for `ContextBuilder` (creation, for_user, with_strategy, with_system_message; MockStore and MockStrategy)
  - **context/context_test.rs**: tests for `Context` and `ContextMetadata` (format_for_model, exceeds_limit)
  - Replaced single `context.rs` with `context/mod.rs` and the three test modules; removed inline `#[cfg(test)] mod tests` from context
- Split memory strategies into independent crates
  - **memory-core** (crates/memory-core): Core types (`MemoryEntry`, `MemoryMetadata`, `MemoryRole`), `MemoryStore` trait, and `StrategyResult` enum. Used by `memory` and `memory-strategies` to avoid circular dependency.
  - **memory-strategies** (crates/memory-strategies): Context building strategies (`RecentMessagesStrategy`, `SemanticSearchStrategy`, `UserPreferencesStrategy`), `ContextStrategy` trait. Depends on `memory-core` and `embedding`. Unit tests in `src/strategies_test.rs`.
  - **memory** crate now depends on `memory-core` and `memory-strategies`; re-exports their public API so existing `memory::*` usage is unchanged. Removed `memory/src/types.rs`, `store.rs`, `strategies.rs`; `context` and `migration` remain in memory.
  - Workspace `Cargo.toml` includes `crates/memory-core` and `crates/memory-strategies`.
- Embedding provider selection for RAG semantic search (OpenAI vs Zhipu AI)
  - `BotConfig`: `embedding_provider` (`openai` | `zhipuai`, default `openai`), `bigmodel_api_key` (from `BIGMODEL_API_KEY` or `ZHIPUAI_API_KEY`)
  - Runner creates `OpenAIEmbedding` or `BigModelEmbedding` per `EMBEDDING_PROVIDER`; errors when `zhipuai` but API key missing
  - `telegram-bot` depends on `bigmodel-embedding`; `.env.example` documents `EMBEDDING_PROVIDER`, `BIGMODEL_API_KEY`, `ZHIPUAI_API_KEY`
  - Config tests: `test_load_config_embedding_provider_zhipuai`, `test_load_config_bigmodel_key_from_zhipuai`
  - Integration tests force `EMBEDDING_PROVIDER=openai` so they do not require `BIGMODEL_API_KEY`
  - `test_embedding_provider_zhipuai_requires_api_key`: asserts init fails when `zhipuai` but no API key
- ai-handlers: unit tests moved to separate file and method doc comments
  - Added `ai-handlers/src/ai_response_handler_test.rs` with all AIQueryHandler unit tests (build_context, format_question_with_context, save_to_memory, build_memory_context); each test has a detailed doc comment describing scenario and assertions
  - Removed inline `#[cfg(test)] mod tests` from `ai_response_handler.rs`
  - Added detailed doc comments to `AIQueryHandler` and its methods (run, handle_query, handle_query_normal/streaming, send_response, log_ai_response, build_context, format_question_with_context, build_memory_context, save_to_memory), including purpose and external interactions
  - Exposed `save_to_memory` and `build_memory_context` as `pub(crate)` for use by the test module; registered `#[cfg(test)] mod ai_response_handler_test` in lib.rs
- docs/rag/context-retrieval-before-reply.md - Documents where the code queries relevant context before AI reply (entry point, build_memory_context, SemanticSearchStrategy, ContextBuilder.build); index entry in docs/rag/README.md
- Process logging for OpenAI embedding operations (openai-embedding crate)
  - Added `tracing` dependency and `#[instrument]` on `embed` and `embed_batch`
  - `embed`: debug at request start (model, text_len); info on success (dimension); warn on API or parse failure
  - `embed_batch`: debug for empty input skip; debug at request start (model, batch_size); info on success (count, dimension); warn on API failure or response count mismatch
- Semantic search over vector store using user question
  - `SemanticSearchStrategy` now uses `EmbeddingService` to embed the user's question and calls `MemoryStore::semantic_search` to retrieve relevant context from the vector store (Lance/SQLite/in-memory)
  - AI handler builds context with `RecentMessagesStrategy`, `SemanticSearchStrategy` (user question → vector search), and `UserPreferencesStrategy`; `ContextBuilder::with_query(question)` passes the question to strategies
  - `AIQueryHandler` accepts `Arc<dyn EmbeddingService>`; telegram-bot runner creates `OpenAIEmbedding` and passes it when building bot components
- Lance vector query verification test (TELEGRAM_BOT_TEST_PLAN 3.2)
  - Added `test_lance_vector_query_verification` in `memory-lance/tests/lance_vector_store_integration_test.rs`
  - Verifies `semantic_search` returns results ordered by similarity; query vector nearest to entry A returns A first
- AI reply flow E2E test (TELEGRAM_BOT_TEST_PLAN 3.4)
  - Added `test_ai_reply_complete_flow` in `telegram-bot/tests/runner_integration_test.rs`
  - Uses `TelegramBot::new_with_memory_store`, `handle_core_message`, and `start_ai_handler` with `MockMemoryStore`
  - Asserts store call count ≥ 2 and query call count ≥ 1; skips when `OPENAI_API_KEY` is not set
- Test-only `handle_core_message` on `TelegramBot` (runner)
  - Accepts `dbot_core::Message` to drive the handler chain without constructing a teloxide `Message`
  - Documented as `#[doc(hidden)]` for integration tests
- Test execution instructions in TELEGRAM_BOT_TEST_PLAN
  - Documented `.env.test` / `.env` usage, single-test commands, and Lance integration test commands
- Create runner integration test file (TELEGRAM_BOT_TEST_PLAN task 1.2)
  - Added `telegram-bot/tests/runner_integration_test.rs` with basic test structure
  - `setup_test_config(temp_dir)` sets env vars and loads `BotConfig` using a temp directory
  - Test `test_setup_test_config_loads` verifies config loading for use in later run_bot integration tests
- Implement test helpers for Telegram bot integration tests (TELEGRAM_BOT_TEST_PLAN task 1.3)
  - Enhanced `setup_test_config` to load `.env.test` / `.env` and isolate DB/memory paths via `TempDir`
  - Added `MockMemoryStore` implementing `memory::MemoryStore` with in-memory `HashMap` backend
  - Added tests `test_setup_test_config_loads` and `test_mock_memory_store_basic_crud_and_queries`
- Add tracing-based logging to memory crate
  - Added `tracing::instrument` and debug logs in `ContextBuilder::build` to trace context construction
  - Added debug logs in `RecentMessagesStrategy`, `SemanticSearchStrategy`, and `UserPreferencesStrategy` to observe memory queries and preference extraction
- Split RAG_SOLUTION.md into modular documentation structure
  - Created docs/rag/ directory with 12 separate documents
  - docs/rag/README.md - Main index and overview
  - docs/rag/architecture.md - Architecture design and core components
  - docs/rag/technical-selection.md - Technology selection and comparison
  - docs/rag/data-flow.md - Data flow and context building
  - docs/rag/configuration.md - Configuration design
  - docs/rag/implementation.md - Implementation plan and roadmap
  - docs/rag/DEVELOPMENT_PLAN.md - Detailed development plan with task tables
  - docs/rag/usage.md - Usage examples and best practices
  - docs/rag/testing.md - Testing strategy
  - docs/rag/cost.md - Cost estimation and analysis
  - docs/rag/future.md - Future extensions
  - docs/rag/references.md - References and resources
  - Updated docs/RAG_SOLUTION.md as a redirect index
- Add ZhipuAI (智谱AI) embedding service support
  - Add ZhipuAI embeddings option in technical-selection.md
  - Add ZhipuAI configuration options in configuration.md
  - Add ZhipuAI cost estimation in cost.md
  - Add ZhipuAI SDK and API references in references.md
  - Model: embedding-2 (1024 dimensions)
  - Python SDK: `pip install zhipuai`

### Removed
- **ai-handlers**: removed unused async AI pipeline
  - Deleted `ai_response_handler.rs` and `ai_response_handler_test.rs`; runner uses only `SyncAIHandler` (in-chain sync AI, returns `Reply` for middleware). No longer exporting `AIQueryHandler`. Docs and README updated to reference `sync_ai_handler.rs` and `SyncAIHandler`.

### Changed
- middleware: unit tests moved to dedicated directory
  - Added `crates/middleware/src/test/`; `mod.rs` declares `memory_middleware_test` and `persistence_middleware_test`
  - Moved `memory_middleware_test.rs` from `src/` to `src/test/memory_middleware_test.rs`
  - Extracted PersistenceMiddleware tests from inline `#[cfg(test)] mod tests` in `persistence_middleware.rs` to `src/test/persistence_middleware_test.rs`
  - `lib.rs` uses `#[cfg(test)] mod test;`; `persistence_middleware.rs` no longer contains inline tests
- Extract `HandlerChain` into a standalone `handler-chain` package
  - Created new package at `crates/handler-chain/`
  - Moved `handler_chain.rs` from `bot-runtime/src/` to `crates/handler-chain/src/`
  - Updated `bot-runtime` to use the new `handler-chain` package
  - All existing tests pass without modification
  - Improves modularity and reusability of the handler chain implementation
- Extract middleware implementations into a standalone `middleware` package
  - Created new package at `crates/middleware/`
  - Moved `middleware.rs`, `memory_middleware.rs`, and `persistence_middleware.rs` from `bot-runtime/src/` to `crates/middleware/src/`
  - Updated `bot-runtime` and `telegram-bot` to use the new `middleware` package
  - All existing tests pass without modification
  - Improves modularity and reusability of middleware implementations

### Documentation
- Improved RAG solution documentation with index-based navigation
- Added detailed code examples for all components
- Added comprehensive testing strategies
- Added cost analysis for different scales
- Added implementation roadmap with time estimates
