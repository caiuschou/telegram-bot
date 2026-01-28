# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
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
