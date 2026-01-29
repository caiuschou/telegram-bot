# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- middleware: unit tests split into independent file
  - Added `crates/middleware/src/memory_middleware_test.rs` with all MemoryMiddleware unit tests (MemoryConfig default, creation, message_to_memory_entry, before/after saving, build_context); tests use InMemoryVectorStore and do not call external services
  - Removed inline `#[cfg(test)] mod tests` from `memory_middleware.rs`
  - Exposed `config`, `message_to_memory_entry`, `reply_to_memory_entry`, and `build_context` as `pub(crate)` for the test module; registered `#[cfg(test)] mod memory_middleware_test` in lib.rs
- SyncAIHandler: AI runs synchronously in the handler chain, returns Reply(text) for middleware to save (Option 1)
  - **ai-handlers**: new `SyncAIHandler` implementing `Handler`; on AI query (reply-to or @mention) builds context, calls AI (normal or streaming), sends reply to Telegram, returns `HandlerResponse::Reply(response_text)` so MemoryMiddleware saves it in `after()`. New module `sync_ai_handler.rs`.
  - **telegram-bot runner**: removed channel and async AI task; `BotComponents` now has `sync_ai_handler: Arc<SyncAIHandler>` instead of `query_sender` and `ai_query_handler`. Chain uses `sync_ai_handler`; user message saved in middleware `before()`, AI reply saved in middleware `after()` when handler returns `Reply(text)`. Removed `start_ai_handler()` and its call from `run_bot`.
  - **telegram-bot tests**: `test_ai_reply_complete_flow` no longer calls `start_ai_handler()` or polls; `handle_core_message` runs the full chain synchronously.
  - **AIDetectionHandler** and **AIQueryHandler** remain in ai-handlers for now (tests, potential other use); runner no longer uses them.
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

### Changed
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
