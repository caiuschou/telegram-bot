# dbot-core 架构

## 概述

Bot 框架核心：领域类型、Bot/Handler/Middleware 抽象、错误与日志；与具体传输（Telegram）无关。

## 设计原则

- Trait 抽象、异步优先（async-trait + tokio）、类型安全（serde）、模块化。

## 模块结构

```
dbot-core/src/
├── lib.rs    # 入口与公开 API 再导出
├── bot.rs    # Bot trait
├── types.rs  # User、Chat、Message、Handler、Middleware、ToCoreUser/ToCoreMessage
├── error.rs  # DbotError、HandlerError、Result
└── logger.rs # init_tracing（控制台 + 文件）
```

## 核心组件

- **Bot trait**：send_message、reply_to、edit_message、send_message_and_return_id；实现见 telegram-bot（TelegramBot）、dbot-telegram（TelegramBotAdapter）。
- **类型**：User、Chat、Message、MessageDirection、HandlerResponse（Continue/Stop/Ignore）；ToCoreUser、ToCoreMessage（teloxide→core）。
- **Handler**：handle(message) → Result<HandlerResponse>；实现见 ai-handlers（SyncAIHandler 等）。
- **Middleware**：before(message)、after(message, response)；实现见 middleware（MemoryMiddleware 等）。
- **错误**：DbotError、HandlerError；Result<T> = Result<T, DbotError>。
- **日志**：init_tracing(log_file_path)。

## 消息流

Telegram API → ToCoreMessage → core::Message → Middleware.before → Handler.handle → Middleware.after → Bot API（发回复）。

详见 [docs/CRATES.md](../docs/CRATES.md) 与源码。
