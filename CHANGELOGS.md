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

### Documentation
- Improved RAG solution documentation with index-based navigation
- Added detailed code examples for all components
- Added comprehensive testing strategies
- Added cost analysis for different scales
- Added implementation roadmap with time estimates
