# Rust Telegram Bot 开发指南

## 概述

使用 Rust 开发 Telegram Bot 的参考方案：技术栈、项目结构、核心模块、部署与测试。

## 技术栈

- **teloxide**：Telegram Bot API 框架（类型安全、中间件、轮询/Webhook）。
- **tokio**：异步运行时。
- **serde / serde_json**：序列化。

依赖示例：`teloxide = { version = "0.12", features = ["macros"] }`、`tokio = { version = "1.35", features = ["full"] }`。

## 项目结构建议

```
telegram-bot/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs
│   ├── handlers.rs
│   ├── services.rs
│   ├── middleware.rs
│   ├── models.rs
│   └── utils.rs
└── README.md
```

## 核心模块

- **Bot 初始化**：Bot::from_env、Dispatcher、依赖注入。
- **消息/命令处理**：Handler、Middleware、状态（Dialogue）。
- **数据库**：SQLite/SQLx、Repository 模式。
- **部署**：Docker、systemd、Kubernetes；日志与监控（tracing、Prometheus）。
- **安全**：环境变量、输入校验、防重放。
- **测试**：单元测试、集成测试、Mock；见 [TELEGRAM_BOT_TEST_PLAN.md](TELEGRAM_BOT_TEST_PLAN.md)。

本项目的具体实现见 dbot-core、dbot-telegram、telegram-bot、ai-handlers；完整示例与代码片段见 git 历史或 [rust-telegram-bot-plan.md](rust-telegram-bot-plan.md)。
