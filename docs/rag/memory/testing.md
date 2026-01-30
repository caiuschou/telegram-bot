# 测试说明

- **单元测试**：memory、memory-strategies、memory-lance 等 crate 的 tests/ 或 #[cfg(test)]；覆盖类型、存储、策略、上下文构建。
- **Mock**：Mock MemoryStore / EmbeddingService 用于 ai-handlers、context 等测试；见各 crate tests/。
- **运行**：`cargo test -p memory`、`cargo test -p memory-strategies`、`cargo test -p memory-lance` 等。
- **集成测试**：telegram-bot/tests/ 需 OPENAI_API_KEY 或 mock；见 [TELEGRAM_BOT_TEST_PLAN.md](../../TELEGRAM_BOT_TEST_PLAN.md)。
