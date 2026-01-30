# 测试策略

- **单元测试**：memory、memory-strategies、memory-lance、ai-handlers 等均有 `tests/` 或 `#[cfg(test)]` 模块；覆盖存储、策略、上下文构建、消息构建。
- **集成测试**：telegram-bot/tests/（如 runner_integration_test）；需 OPENAI_API_KEY 或 mock。
- **运行**：`cargo test -p memory`、`cargo test -p ai-handlers`、`cargo test -p telegram-bot`；Bot 测试环境见 [TELEGRAM_BOT_TEST_PLAN.md](../TELEGRAM_BOT_TEST_PLAN.md)。
