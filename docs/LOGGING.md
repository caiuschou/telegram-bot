# Telegram Bot 日志

- **目录**: `logs/`（如 `telegram-bot.log`）。
- **格式**: `[timestamp] [level] message`。级别：TRACE / DEBUG / INFO / WARN / ERROR。
- **配置**: 环境变量 `RUST_LOG`（例：`RUST_LOG=info`、`RUST_LOG=debug`）。框架库（teloxide、reqwest 等）默认 warn，如需调试可设 `RUST_LOG=debug,teloxide=debug`。
- **轮转**: 当前为追加写入；生产建议按大小或时间做轮转与保留策略。
