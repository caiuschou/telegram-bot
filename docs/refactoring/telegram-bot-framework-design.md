# Telegram Bot 框架设计：从本库抽离的专用框架

## 1. 目标

从当前 dbot 库中抽离出一套**专门处理 Telegram Bot 的框架**，达到：

- **简化开发**：最少几行代码即可跑起一个 Bot（REPL + 消息处理）。
- **边界清晰**：框架只负责「Telegram 接入 + 消息链 + 扩展点」，不捆绑 AI/持久化/记忆。
- **易扩展**：通过 Handler / Middleware 扩展，AI、持久化、记忆等以「插件」形式接入。
- **可测试**：核心用 `dbot_core::Message` 与 trait 抽象，便于单测与集成测试（可换 API URL 指向 mock）。

## 2. 框架定位

### 2.1 框架做什么（In Scope）

| 能力 | 说明 |
|------|------|
| **Telegram 接入** | 基于 teloxide 的 long polling（REPL），token / 可选 API URL 配置。 |
| **消息标准化** | 将 teloxide `Message` 转为 `dbot_core::Message`，统一后续处理。 |
| **处理链** | 按顺序执行 Middleware（before）→ Handler 列表 → Middleware（after），与现有 handler-chain 一致。 |
| **发送抽象** | 使用 `dbot_core::Bot` trait 发消息/编辑消息，便于测试与替换实现。 |
| **最小配置** | 仅：token、可选 API URL、可选日志路径。 |
| **运行入口** | 一行 `run().await` 或 `start().await` 启动 REPL。 |

### 2.2 框架不做什么（Out of Scope，由上层/插件提供）

| 能力 | 说明 |
|------|------|
| **持久化** | 消息存库、Repository 等 → 由应用或 `middleware` crate 的 `PersistenceMiddleware` 提供。 |
| **记忆 / RAG** | 向量存储、上下文构建、语义检索 → 由 `memory`、`ai-handlers` 等提供。 |
| **AI 调用** | LLM、流式回复 → 由 `ai-client`、`ai-handlers` 提供，作为「一个 Handler」接入链。 |
| **业务配置** | 数据库 URL、AI 模型、embedding 等 → 由使用框架的应用自己加载。 |

即：**框架 = Telegram 适配 + 链式处理 + 扩展点**；业务与可选能力 = **在链上挂 Handler/Middleware**。

## 3. 目标架构

### 3.1 分层与依赖

```
┌─────────────────────────────────────────────────────────────────┐
│  应用层（本库内示例：当前 telegram-bot 主应用）                      │
│  配置(含 DB/AI/记忆) + 组装 Middleware/Handler + 调用框架 run()   │
└──────────────────────────────┬──────────────────────────────────┘
                                │ 使用
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  Telegram Bot 框架层（新：dbot-telegram 或 telegram-framework）   │
│  • TelegramRunner / BotBuilder                                   │
│  • 最小配置（token, api_url?, log?）                              │
│  • 适配器：teloxide Message → core::Message，Bot trait 实现       │
│  • 启动 REPL，每条消息：转换 → HandlerChain.handle(core_msg)      │
└──────────────────────────────┬──────────────────────────────────┘
                                │ 依赖
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  核心层（现有）                                                    │
│  dbot-core (Message, Handler, Middleware, Bot)                   │
│  handler-chain (HandlerChain)                                     │
└─────────────────────────────────────────────────────────────────┘
```

- **框架 crate** 只依赖：`dbot-core`、`handler-chain`、`teloxide`、`tracing` 等，**不依赖** `storage`、`memory`、`ai-handlers`、`openai-client`。
- **应用**（如当前 telegram-bot）依赖框架 + 若干「插件」crate（middleware、storage、ai-handlers 等），在应用内组装链并调用 `framework.run(config, chain).await`。

### 3.2 框架 Crate 命名与位置

两种可选方式（二选一即可）：

| 方案 | Crate 名 | 位置 | 说明 |
|------|----------|------|------|
| A | `dbot-telegram` | `crates/dbot-telegram/` | 与 dbot-core、handler-chain 同属「dbot 生态」，命名统一。 |
| B | `telegram-bot-framework` | 根目录或 `crates/telegram-bot-framework/` | 名字直接表达「Telegram Bot 框架」，便于单独开源或独立文档。 |

下文统一用 **`dbot-telegram`** 作为框架 crate 名；若你更希望「框架」独立品牌，可整体替换为 B。

## 4. API 设计（简化开发）

### 4.1 最小可用：仅 Token + 一个处理函数

目标：用户写最少代码就能跑一个 echo/clock 类 Bot。

```rust
// 示例：仅框架，无链、无 middleware
use dbot_telegram::{TelegramRunner, run_with_handler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dbot_core::init_tracing("logs/bot.log")?;
    let token = std::env::var("BOT_TOKEN")?;
    run_with_handler(token, |ctx| async move {
        if let Some(text) = ctx.message.text() {
            ctx.reply(format!("Echo: {}", text)).await?;
        }
        Ok(())
    }).await
}
```

这里 `run_with_handler` 是框架提供的便捷入口：内部会创建 REPL、将 teloxide 消息转成某种「上下文」类型、调用用户闭包。若希望**完全**不暴露 teloxide 类型，可让 `ctx` 只带 `dbot_core::Message` + 一个 `reply(text)`（内部用 Bot trait）。

### 4.2 推荐方式：Builder + HandlerChain（与现有模型一致）

与当前 dbot 的「链」模型一致，便于接入现有 Handler/Middleware，同时简化「配置 + 启动」的样板代码。

```rust
use dbot_telegram::TelegramRunner;
use dbot_core::{Handler, Message, HandlerResponse};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dbot_core::init_tracing("logs/bot.log")?;
    let token = std::env::var("BOT_TOKEN")?;

    let chain = handler_chain::HandlerChain::new()
        // .add_middleware(persistence_middleware)  // 可选：由应用提供
        // .add_middleware(memory_middleware)       // 可选
        .add_handler(Arc::new(MyEchoHandler));

    TelegramRunner::new(token)
        .api_url(std::env::var("TELEGRAM_API_URL").ok())  // 可选，测试可指向 mock
        .log_file("logs/bot.log")                         // 可选
        .handler_chain(chain)
        .run()
        .await?;
    Ok(())
}
```

- **TelegramRunner**：持有 token、可选 api_url、可选 log_file、以及一个 `HandlerChain`。
- **run()**：初始化 teloxide Bot、可选设置 get_me 缓存、启动 REPL；每收到一条消息：用适配器转成 `dbot_core::Message`，调用 `chain.handle(&msg)`，无需用户再写 REPL 闭包和转换逻辑。

这样「简化」体现在：用户只关心「配 token + 组链 + run」，而不是自己写 teloxide::repl、ToCoreMessage、错误处理等。

### 4.3 框架提供的类型（建议）

| 类型 | 职责 |
|------|------|
| **TelegramConfig** | 仅含 token、api_url、log_file 等最小配置；可从 env 加载（如 `TelegramConfig::from_env()`）。 |
| **TelegramRunner** | Builder：`new(token)`，`.api_url()`, `.log_file()`, `.handler_chain(chain)`，`.run().await`。 |
| **TelegramBotAdapter** | 实现 `dbot_core::Bot`，内部包装 `teloxide::Bot`，供链中 Handler 发消息/编辑消息（需 Bot trait 有 `edit_message` 时由框架实现）。 |
| **ToCoreMessage 实现** | 从 `teloxide::Message` 到 `dbot_core::Message` 的转换（可放在框架 crate 的 `adapters` 模块，与当前 telegram-bot 的 `TelegramMessageWrapper` 逻辑一致）。 |

可选：若提供「最小可用」API，可再增加：

- **run_with_handler(token, F)**：F 为 `async fn(ctx: &RunContext) -> Result<()>`，RunContext 提供 `message: &Message`、`reply(text)` 等，内部创建单 Handler 的链并 run。

### 4.4 配置分离：框架配置 vs 应用配置

- **框架配置（TelegramConfig）**：只包含框架运行所需字段，例如：
  - `bot_token: String`
  - `telegram_api_url: Option<String>`
  - `log_file: Option<String>`
  - （可选）`bot_username_cache: bool` 等与 REPL 行为相关的开关。

- **应用配置**：数据库 URL、AI 模型、记忆策略、embedding 等，留在**应用**侧（如当前 `BotConfig`），框架不解析、不依赖。应用在 `main` 里先加载自己的配置，再构造 Handler/Middleware，最后调用 `TelegramRunner::new(config.bot_token).handler_chain(chain).run().await`，必要时把应用配置通过闭包或共享状态传给 Handler。

这样「专门处理 Telegram Bot 的框架」只关心 Telegram 与链，不关心业务配置。

## 5. 从本库的抽离方式

### 5.1 从当前 telegram-bot 中迁到框架的代码

| 现有位置 | 迁入框架后 | 说明 |
|----------|------------|------|
| `telegram-bot/src/adapters.rs` | `dbot-telegram/src/adapters.rs` | Teloxide Message → core Message，Bot trait 实现（发送/编辑）。 |
| `telegram-bot/src/runner.rs` 中「REPL + 转换 + 调用链」的逻辑 | `dbot-telegram/src/runner.rs` | 只保留：建 Bot、可选 get_me、repl 里对每条消息 to_core + chain.handle；不包含 persistence/memory/AI 的组装。 |
| `telegram-bot/src/config.rs` 中与「Telegram + 日志」相关的字段 | `dbot-telegram/src/config.rs` 的 `TelegramConfig` | 仅 token、api_url、log_file；其余（database_url、ai_*、memory_*）留在应用侧。 |

### 5.2 保留在应用（当前 telegram-bot）的代码

| 内容 | 说明 |
|------|------|
| 业务配置 `BotConfig` | 含 database_url、openai_*、ai_*、memory_*、embedding_* 等；从 env 加载，传给自己的 Middleware/Handler。 |
| 组件组装 | 创建 `MessageRepository`、`PersistenceMiddleware`、`MemoryMiddleware`、`SyncAIHandler`、embedding、memory store 等，组成 `HandlerChain`。 |
| `run_bot(config)` 入口 | 先 `init_tracing`（或用框架提供的可选日志初始化），再 `TelegramRunner::new(config.bot_token).api_url(config.telegram_api_url).handler_chain(chain).run().await`，其中 `chain` 由应用用 config 构建。 |

### 5.3 依赖关系（拆分后）

- **dbot-telegram**（新）：依赖 `dbot-core`、`handler-chain`、`teloxide`、`tracing`、`anyhow` 等；**不依赖** storage、memory、ai-handlers、openai-client、middleware（middleware 的 trait 来自 dbot-core，实现由应用或现有 middleware crate 提供）。
- **telegram-bot**（现有应用）：依赖 `dbot-telegram`、`dbot-core`、`handler-chain`、`middleware`、`storage`、`memory`、`ai-handlers` 等；负责读取 `BotConfig`、组装链、调用 `TelegramRunner::...::run().await`。

这样「专门处理 Telegram Bot 的框架」就落在单一 crate，应用按需拉取插件式依赖并组装。

## 6. 使用示例对比

### 6.1 仅用框架：Echo Bot（无持久化、无 AI）

```rust
// 使用框架后的 echo 示例
use dbot_telegram::TelegramRunner;
use handler_chain::HandlerChain;
use std::sync::Arc;

struct EchoHandler;
#[async_trait::async_trait]
impl dbot_core::Handler for EchoHandler {
    async fn handle(&self, msg: &dbot_core::Message) -> dbot_core::Result<dbot_core::HandlerResponse> {
        // 需要拿到 Bot 才能发消息：可通过 Handler 构造时注入 Arc<dyn Bot>，或通过上下文传递
        // 这里仅作示意，实际由框架在链外持有 Bot 并注入到能 Reply 的 Handler
        Ok(dbot_core::HandlerResponse::Continue)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dbot_core::init_tracing("logs/echo.log")?;
    let chain = HandlerChain::new().add_handler(Arc::new(EchoHandler));
    TelegramRunner::from_env()
        .handler_chain(chain)
        .run()
        .await?;
    Ok(())
}
```

说明：当前 `Handler` 只有 `message`，没有「发消息」能力；发消息要通过实现 `Bot` 的实例。所以要么：  
- 在框架里提供「带 Bot 的 Handler 上下文」（例如通过 thread-local 或 run 时的上下文对象注入 Bot），要么  
- Echo 这类简单场景用 `run_with_handler`，闭包内拿到 `ctx.reply()`。  
设计文档里可写：**链式 Handler 若需发消息，则持有 `Arc<dyn Bot>`（由应用在组装链时注入）**；框架只保证在 run 时把同一个 Bot 实现体传给链（例如通过 Middleware 或 Handler 的构造函数注入）。

### 6.2 当前完整应用：框架 + 持久化 + 记忆 + AI

```rust
// telegram-bot 主应用（保留现有逻辑，改为使用框架入口）
use dbot_telegram::TelegramRunner;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = telegram_bot::BotConfig::load(None)?;
    std::fs::create_dir_all("logs")?;
    dbot_core::init_tracing(&config.log_file)?;

    let chain = telegram_bot::build_handler_chain(&config).await?;  // 现有组装逻辑

    TelegramRunner::new(config.bot_token)
        .api_url(config.telegram_api_url)
        .handler_chain(chain)
        .run()
        .await?;
    Ok(())
}
```

`build_handler_chain` 内部仍会创建 PersistenceMiddleware、MemoryMiddleware、SyncAIHandler 等，与现在一致，只是「REPL 与消息驱动」交给框架完成。

## 7. 与「Bot / AI 拆分」方案的关系

- **Crate 拆分方案（Bot vs AI）**：解决的是「AI 层与 Bot 层解耦、ai-client 独立、ai-handlers 不依赖 teloxide」等问题。
- **本设计（Telegram Bot 框架）**：解决的是「把 Telegram 接入与链式处理抽成可复用框架、简化开发、配置与插件边界清晰」等问题。

两者可并行推进：

1. 先落 **Telegram Bot 框架**（新 crate `dbot-telegram`，从 telegram-bot 抽 REPL + 适配器 + 最小配置），当前 telegram-bot 应用改为依赖该框架并只做「组装链 + 业务配置」。
2. 再按 **Bot/AI 拆分方案** 做 ai-client、Bot trait 的 edit_message、ai-handlers 只依赖 Bot + LlmClient 等。

最终：**框架层（dbot-telegram）** 只依赖 **dbot-core + handler-chain**，不依赖 AI/存储；**应用** 在框架之上挂「持久化 / 记忆 / AI」等 Handler 与 Middleware，形成你现在看到的「完整 Bot」能力。

## 8. 小结

| 维度 | 设计要点 |
|------|----------|
| **目标** | 专门处理 Telegram Bot 的框架，简化启动与扩展，不捆绑 AI/持久化/记忆。 |
| **范围** | Telegram 接入、消息转 core、HandlerChain 执行、Bot trait 发送；配置仅 token/api_url/log。 |
| **简化** | Builder API（TelegramRunner）+ 可选 `run_with_handler`；应用只组链 + run。 |
| **抽离** | 从当前 telegram-bot 迁出适配器、REPL+链调用、最小配置到新 crate `dbot-telegram`。 |
| **扩展** | 通过 Handler/Middleware 挂载；持久化、记忆、AI 作为应用侧或独立 crate 的插件。 |
| **与 Bot/AI 拆分** | 互补：框架管「Telegram + 链」，AI 拆分方案管「AI 层与 Bot 层解耦」。 |

按此设计，你可以先实现 **dbot-telegram** 框架 crate，再把现有 **telegram-bot** 改为基于该框架的应用；后续再推进 ai-client 与 Bot/AI 拆分，两者不冲突。

---

## 相关文档

- [Crate 与文件索引](../CRATES.md)：每个 crate 及对应文件与描述，遵循「每个 crate 尽量简单」。
- [开发计划](DEVELOPMENT_PLAN.md)：按阶段与任务表执行的详细开发计划（P0～P3、验收与测试）。
