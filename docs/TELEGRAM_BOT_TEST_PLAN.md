# Telegram Bot 集成测试方案

## 概述

验证 `run_bot` 与 AI 回复流程；重点：Lance 向量存储与查询、消息持久化、上下文构建与 LLM 调用。

## 开发计划（摘要）

| 阶段 | 内容 | 状态 |
|------|------|------|
| 1 | 测试依赖、runner_integration_test、MockMemoryStore、Mock Telegram API | 已完成 |
| 2 | 重构 runner（BotComponents、TelegramBot::new、handle_message、start_ai_handler） | 已完成 |
| 3 | Lance 存储/查询验证、真实 OpenAI E2E（可选）、AI 回复流程测试 | 部分完成 |
| 4 | .env.test.example、Lance 文档、执行说明 | 已完成 |
| 5 | 全量测试通过、覆盖率与性能（可选） | 进行中 |

## 测试场景

| 用例 | 验证点 |
|------|--------|
| AI 回复完整流程 | Bot 初始化、Lance 连接、消息持久化与向量化、语义检索、上下文传给 AI、回复发送与存库 |

## 测试执行

- **推荐**：复制 `.env.test.example` 为 `.env.test` 或 `.env`，填入 `OPENAI_API_KEY`（仅 E2E 需要）；未设置时相关 E2E 自动跳过。
- **命令**：`cd telegram-bot && cargo test --test runner_integration_test`；仅 smoke/Mock：`cargo test --test runner_integration_test -- --skip ai_reply`（按实际 test 名称调整）。
- **环境变量**：OPENAI_API_KEY、BOT_TOKEN（测试用）、MEMORY_STORE_TYPE、MEMORY_LANCE_PATH（或临时目录）；见 telegram-bot/.env.test.example。

## 技术实现

- **依赖**：mockall、mockito、tempfile、tokio-test（见 telegram-bot/Cargo.toml [dev-dependencies]）。
- **Mock**：MockMemoryStore、mockito 提供 getMe/sendMessage；Lance 使用临时目录。
- **向量验证**：memory-lance 集成测试（test_lance_vector_store_verification、test_semantic_search_regression_golden_cases 等）。

详细步骤与代码见 telegram-bot/tests/runner_integration_test.rs；覆盖率见 [TEST_COVERAGE.md](TEST_COVERAGE.md)。
