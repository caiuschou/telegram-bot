# 合并成一个 Crate 方案

将 **dbot-core**、**handler-chain**、**dbot-telegram**、**telegram-bot** 四个 crate 合并为单一 crate（以 **telegram-bot** 为合并后的包名和目录）。

---

## 1. 目标与依赖关系

| 原 Crate       | 合并后位置            | 对外使用方式 |
|----------------|-----------------------|--------------|
| dbot-core      | `telegram-bot::core`  | `use telegram_bot::{Handler, Bot, Message, …}` |
| handler-chain  | `telegram-bot::chain` | `use telegram_bot::HandlerChain` |
| dbot-telegram  | `telegram-bot::telegram` | `use telegram_bot::run_repl` |
| telegram-bot   | 根模块 + config/components/runner | `use telegram_bot::{run_bot, BotConfig, BotComponents, …}` |

合并后：**只有一个 crate**（telegram-bot），框架层（core + chain + telegram）与应用层（config + components + run_bot）同包；其他 crate（dbot-cli、dbot-llm-cli、handlers、llm-handlers 等）只依赖 **telegram-bot**。

---

## 2. 模块结构

```
telegram-bot/
├── Cargo.toml          # 合并后的依赖（见第 3 节）
└── src/
    ├── lib.rs          # 对外 re-export（见第 4 节）
    ├── core/           # 原 dbot-core
    │   ├── mod.rs
    │   ├── types.rs    # Message, User, Chat, HandlerResponse, Handler, Bot, ToCore*
    │   ├── error.rs
    │   ├── bot.rs      # Bot trait + TelegramBot 实现
    │   └── logger.rs
    ├── chain.rs        # 原 handler-chain：HandlerChain（或 chain/mod.rs）
    ├── telegram/       # 原 dbot-telegram
    │   ├── mod.rs
    │   ├── adapters.rs # ToCoreMessage 等
    │   └── runner.rs   # run_repl
    ├── config/         # 原 telegram-bot config
    │   ├── mod.rs
    │   ├── base.rs
    │   ├── bot_config.rs
    │   └── extensions.rs
    ├── components.rs   # create_memory_stores, build_bot_components, build_handler_chain
    ├── runner.rs       # run_bot
    └── telegram_impl.rs
```

- **core**：仅类型与 trait（Handler, Bot, Message, HandlerResponse 等），以及 error、logger、bot（含 TelegramBot）。
- **chain**：整块来自 handler-chain，仅依赖 `crate::core`。
- **telegram**：适配器 + run_repl，依赖 `crate::core` 与 `crate::chain`。
- **config / components / runner / telegram_impl**：原 telegram-bot 应用层，依赖 core、chain、telegram 及 memory、handlers 等。

---

## 3. 合并后的 Cargo.toml

在 **telegram-bot** 的 `Cargo.toml` 上合并，**去掉**对 `dbot-core`、`handler-chain`、`dbot-telegram` 的 path 依赖，其余保留并去重：

```toml
[package]
name = "telegram-bot"
version = "0.1.0"
edition = "2021"

[features]
default = []
lance = ["memory-lance"]

[dependencies]
# 原 dbot-core
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }

# 原 dbot-telegram + telegram-bot
teloxide = { version = "0.17", features = ["macros"] }
tokio = { version = "1.35", features = ["full"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
dotenvy = "0.15"

# 原 telegram-bot 应用层
storage = { path = "../storage" }
memory = { path = "../memory" }
memory-inmemory = { path = "../crates/memory/memory-inmemory" }
memory-sqlite = { path = "../crates/memory/memory-sqlite" }
memory-lance = { path = "../crates/memory/memory-lance", optional = true }
handlers = { path = "../handlers" }
embedding = { path = "../crates/embedding/embedding" }
openai-embedding = { path = "../crates/embedding/openai-embedding" }
bigmodel-embedding = { path = "../crates/embedding/bigmodel-embedding" }

[dev-dependencies]
mockall = "0.14"
mockito = "1.7"
tempfile = "3.24"
tokio-test = "0.4"
uuid = { version = "1.6", features = ["v4", "serde"] }
serial_test = "3.0"
dbot-llm-cli = { path = "../dbot-llm-cli" }
```

说明：不再依赖 `dbot-core`、`handler-chain`、`dbot-telegram`，其功能在本 crate 的 `core` / `chain` / `telegram` 中实现。

---

## 4. 对外 API（lib.rs re-export）

在 **src/lib.rs** 中统一导出，使 dbot-cli、dbot-llm-cli 等只依赖本 crate：

```rust
// 原 dbot-core
pub mod core;
pub use core::{
    Bot, Handler, HandlerResponse, Message, User, Chat, MessageDirection,
    ToCoreMessage, ToCoreUser,
};
pub use core::error::{DbotError, HandlerError, Result};
pub use core::logger::init_tracing;
pub use core::bot::TelegramBot;

// 原 handler-chain
pub mod chain;
pub use chain::HandlerChain;

// 原 dbot-telegram
pub mod telegram;
pub use telegram::run_repl;

// 原 telegram-bot 应用
pub mod config;
pub use config::{AppExtensions, BotConfig};
pub use components::{build_bot_components, build_handler_chain, create_memory_stores, BotComponents};
pub use runner::run_bot;
```

---

## 5. 引用与 workspace 调整

### 5.1 本 crate 内部

- 原 `dbot_core::*` → `crate::core::*` 或 `crate::*`（若已 re-export）
- 原 `handler_chain::HandlerChain` → `crate::chain::HandlerChain` 或 `crate::HandlerChain`
- 原 `dbot_telegram::run_repl` → `crate::telegram::run_repl` 或 `crate::run_repl`

### 5.2 其他 crate

将原对 **dbot-core** / **handler-chain** / **dbot-telegram** 的依赖改为只依赖 **telegram-bot**：

- **handlers**：`dbot-core` → `telegram-bot`（Handler, Message, HandlerResponse 等）
- **llm-handlers**：`dbot-core` → `telegram-bot`
- **dbot-cli**：若引用 BotConfig 等，保留/改为 `telegram-bot`
- **dbot-llm-cli**：`dbot_telegram`、`telegram_bot` 等统一为 `telegram_bot`

示例：`use dbot_core::Handler` → `use telegram_bot::Handler`。

### 5.3 根 Cargo.toml (workspace)

- 从 `members` 中**移除**：`dbot-core`、`handler-chain`、`dbot-telegram`
- 保留 **telegram-bot** 作为合并后的唯一库

---

## 6. 实施顺序

1. **迁入代码**：在 telegram-bot 下新建 `src/core`、`src/chain`、`src/telegram`，将 dbot-core 按文件迁入 `core/`，handler-chain 迁入 `chain.rs`（或 `chain/mod.rs`），dbot-telegram 迁入 `telegram/`；原 telegram-bot 的 config、components、runner 等保留，内部 `use` 改为 `crate::core` / `crate::chain` / `crate::telegram`。
2. **合并 Cargo.toml**：去掉对 dbot-core、handler-chain、dbot-telegram 的 path 依赖，保留并去重其余依赖（见第 3 节）。
3. **统一导出**：在 lib.rs 中按第 4 节写好 `pub mod` 与 `pub use`，保证 Handler、Bot、Message、HandlerChain、run_repl、run_bot、BotConfig、BotComponents 等一次导出。
4. **全局替换**：在 handlers、llm-handlers、dbot-cli、dbot-llm-cli、storage 等中，将 `dbot-core`、`handler-chain`、`dbot-telegram` 的依赖与 `use` 改为对 **telegram-bot** 的引用。
5. **移除旧成员**：在根 Cargo.toml 的 workspace `members` 中删除 dbot-core、handler-chain、dbot-telegram。
6. **验证**：执行 `cargo build --workspace` 与 `cargo test --workspace`，按报错修正残留路径或引用。

---

## 7. 小结

合并完成后，框架层（core + chain + telegram）与应用层（config + components + run_bot）同属 **telegram-bot** 一个 crate；其他包仅依赖 telegram-bot 即可获得 Handler、HandlerChain、run_repl、run_bot、BotConfig 等全部能力。
