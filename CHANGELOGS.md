# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
