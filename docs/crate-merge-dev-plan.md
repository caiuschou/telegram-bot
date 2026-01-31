# Crate 合并开发计划（细粒度步骤）

本文档将「合并 dbot-core、handler-chain、dbot-telegram、telegram-bot 为一个 crate」拆成可逐项执行的小步骤。每步完成后建议执行 `cargo build -p telegram-bot` 或 `cargo build --workspace` 做快速验证（步骤中会标明）。

**前置**：已阅读 [crate-merge-plan.md](./crate-merge-plan.md)，了解目标结构与对外 API。

---

## 阶段一：在 telegram-bot 内建立 core 模块（原 dbot-core）

### 1.1 创建 core 目录与 error

1. 在 `telegram-bot/src/` 下新建目录 `core/`。
2. 将 `dbot-core/src/error.rs` 完整复制到 `telegram-bot/src/core/error.rs`（无需改引用，该文件无 `crate::`）。
3. 在 `telegram-bot/src/core/` 下新建 `mod.rs`，内容仅一行：`pub mod error;`。
4. 执行 `cargo build -p telegram-bot`，应失败（因 lib.rs 尚未声明 `mod core`）；暂时在 `telegram-bot/src/lib.rs` 顶部增加 `mod core;`，再构建直到通过。

### 1.2 迁入 types

5. 将 `dbot-core/src/types.rs` 复制到 `telegram-bot/src/core/types.rs`。
6. 在 `core/types.rs` 中，将所有 `crate::error::` 替换为 `crate::core::error::`（Handler trait 内约 3 处）。
7. 在 `telegram-bot/src/core/mod.rs` 中增加：`pub mod types;`，并视需要 `pub use types::*;` 或按需 re-export。
8. `cargo build -p telegram-bot`。

### 1.3 迁入 bot

9. 将 `dbot-core/src/bot.rs` 复制到 `telegram-bot/src/core/bot.rs`。
10. 在 `core/bot.rs` 中：`crate::error::` → `crate::core::error::`，`crate::types::` → `crate::core::types::`（或 `super::types::`）。
11. 在 `core/mod.rs` 中增加 `pub mod bot;` 及相应 `pub use`。
12. `cargo build -p telegram-bot`。

### 1.4 迁入 logger

13. 将 `dbot-core/src/logger.rs` 复制到 `telegram-bot/src/core/logger.rs`。
14. 若 logger 中有 `crate::` 引用，改为 `crate::core::*`；若无则不改。
15. 在 `core/mod.rs` 中增加 `pub mod logger;` 及 `pub use logger::init_tracing;`。
16. `cargo build -p telegram-bot`。

### 1.5 统一 core 对外导出

17. 整理 `telegram-bot/src/core/mod.rs`：对外暴露 `pub use error::{DbotError, HandlerError, Result};`、`pub use types::{...};`、`pub use bot::{Bot, TelegramBot, parse_message_id};`、`pub use logger::init_tracing;`，使后续链与 telegram 模块只依赖 `crate::core`。
18. `cargo build -p telegram-bot`。

---

## 阶段二：在 telegram-bot 内建立 chain 模块（原 handler-chain）

### 2.1 迁入 HandlerChain

19. 将 `handler-chain/src/lib.rs` 全文复制到 `telegram-bot/src/chain.rs`（或 `chain/mod.rs`，二选一；以下按单文件 `chain.rs`）。
20. 在 `chain.rs` 顶部：`use dbot_core::{...}` 改为 `use crate::core::{Handler, HandlerResponse, Message, Result}`（或 `use crate::core::*` 若 core 已统一 re-export）。
21. 在 `telegram-bot/src/lib.rs` 中增加 `mod chain;`（若之前仅加了 `mod core;`，保留两者）。
22. `cargo build -p telegram-bot`。

### 2.2 迁移 handler-chain 的测试（可选）

23. 将 `handler-chain/tests/handler_chain_test.rs` 复制到 `telegram-bot/tests/handler_chain_test.rs`（或保留在 handler-chain 直至删除该 crate 前再移）。若已复制：把其中 `dbot_core::`、`handler_chain::` 改为 `telegram_bot::`（或 `crate::`），并确保测试使用 `telegram_bot::HandlerChain`、`telegram_bot::Message` 等。
24. `cargo test -p telegram-bot handler_chain`（若已迁移该测试）。

---

## 阶段三：在 telegram-bot 内建立 telegram 模块（原 dbot-telegram）

### 3.1 创建 telegram 目录与 adapters

25. 在 `telegram-bot/src/` 下新建目录 `telegram/`。
26. 将 `dbot-telegram/src/adapters.rs` 复制到 `telegram-bot/src/telegram/adapters.rs`。
27. 在 `telegram/adapters.rs` 中：`dbot_core::` → `crate::core::`（或 `crate::core` 并改用 `use crate::core::{Chat, Message, ...}`）。
28. 在 `telegram-bot/src/telegram/mod.rs` 中写 `mod adapters;` 和 `pub use adapters::{TelegramMessageWrapper, TelegramUserWrapper};`。
29. `cargo build -p telegram-bot`。

### 3.2 迁入 bot_adapter

30. 将 `dbot-telegram/src/bot_adapter.rs` 复制到 `telegram-bot/src/telegram/bot_adapter.rs`。
31. `dbot_core::` → `crate::core::`（或 `crate::core`）；`dbot_core::DbotError` → `crate::core::DbotError`。
32. 在 `telegram/mod.rs` 中增加 `mod bot_adapter;` 和 `pub use bot_adapter::TelegramBotAdapter;`。
33. `cargo build -p telegram-bot`。

### 3.3 迁入 config 与 runner

34. 将 `dbot-telegram/src/config.rs` 复制到 `telegram-bot/src/telegram/config.rs`，若有对 dbot_core 的引用改为 `crate::core`。
35. 将 `dbot-telegram/src/runner.rs` 复制到 `telegram-bot/src/telegram/runner.rs`。
36. 在 `telegram/runner.rs` 中：`dbot_core::ToCoreMessage` → `crate::core::ToCoreMessage`；`handler_chain::HandlerChain` → `crate::chain::HandlerChain`（或 `crate::HandlerChain` 若已 re-export）。
37. 在 `telegram/mod.rs` 中增加 `mod config;`、`mod runner;` 及 `pub use config::TelegramConfig;`、`pub use runner::run_repl;`。
38. 在 `telegram-bot/src/lib.rs` 中增加 `mod telegram;`。
39. `cargo build -p telegram-bot`。

---

## 阶段四：telegram-bot 现有代码改用内部模块

### 4.1 修改 components

40. 打开 `telegram-bot/src/components.rs`：`use dbot_core::Handler` → `use crate::core::Handler`（或 `use crate::Handler` 若稍后在 lib 中 re-export）；`use handler_chain::HandlerChain` → `use crate::chain::HandlerChain`（或 `use crate::HandlerChain`）。
41. `cargo build -p telegram-bot`。

### 4.2 修改 runner

42. 打开 `telegram-bot/src/runner.rs`：`dbot_core::` → `crate::core::`（或 `crate::`）；`dbot_telegram::` → `crate::telegram::`（或 `crate::`）；`handler_chain::` → `crate::chain::`（或 `crate::`）。即 `Handler`、`init_tracing`、`Message as CoreMessage`、`ToCoreMessage` 来自 core；`run_repl`、`TelegramMessageWrapper` 来自 telegram；`HandlerChain` 来自 chain。
43. `cargo build -p telegram-bot`。

### 4.3 修改 telegram_impl

44. 打开 `telegram-bot/src/telegram_impl.rs`：`dbot_core::` → `crate::core::`（或 `crate::`），包括 `Bot as CoreBot`、`Chat`、`Message`、`Result`、`DbotError`。
45. `cargo build -p telegram-bot`。

### 4.4 统一 lib.rs 对外 API

46. 打开 `telegram-bot/src/lib.rs`：删除原先对 `dbot_telegram`、`handler_chain` 的 re-export；改为从本 crate 模块导出。例如：
    - `pub use core::*` 或按需 `pub use core::{Bot, Handler, Message, ...};`
    - `pub use chain::HandlerChain;`
    - `pub use telegram::{run_repl, TelegramBotAdapter, TelegramMessageWrapper, TelegramUserWrapper, TelegramConfig};`
    - 保留 `pub use telegram_impl::TelegramBot;` 若仍使用；或改为从 core 导出 `TelegramBot`（若最终采用 core 内的实现）。按当前设计：core 保留 Bot trait + TelegramBot，telegram 保留 TelegramBotAdapter；应用层可继续用 `telegram_impl::TelegramBot` 或 core 的 TelegramBot，需与 runner/components 一致。
47. 确保 `pub use config::...`、`pub use components::...`、`pub use runner::run_bot` 等不变。
48. `cargo build -p telegram-bot`。

### 4.5 修正 components 对 HandlerChain 的构造

49. 在 `telegram-bot/src/components.rs` 的 `build_handler_chain` 中，确保 `HandlerChain::new()` 来自 `crate::chain::HandlerChain`（已通过 use 或 re-export 解决）。无需再改逻辑，仅确认编译通过。
50. `cargo build -p telegram-bot`。

---

## 阶段五：telegram-bot 的 Cargo.toml 去掉对三 crate 的依赖

### 5.1 移除 path 依赖并补齐直接依赖

51. 打开 `telegram-bot/Cargo.toml`：删除 `dbot-core = { path = "../dbot-core" }`、`handler-chain = { path = "../handler-chain" }`、`dbot-telegram = { path = "../dbot-telegram" }`。
52. 对比 `dbot-core/Cargo.toml`、`handler-chain/Cargo.toml`：若 telegram-bot 尚未声明的依赖则补上（如 `thiserror`、`tracing-subscriber` 等，telegram-bot 若已有可跳过）。
53. `cargo build -p telegram-bot`。

### 5.2 处理 dev-dependencies

54. 在 `telegram-bot/Cargo.toml` 的 `[dev-dependencies]` 中删除对 `dbot-core` 的 path 依赖（若有）。保留对 `dbot-llm-cli` 的依赖用于集成测试（若需要）。
55. `cargo build -p telegram-bot` 与 `cargo test -p telegram-bot`。

---

## 阶段六：其他 crate 改为依赖 telegram-bot

### 6.1 handlers

56. 打开 `handlers/Cargo.toml`：将 `dbot-core = { path = "../dbot-core" }` 改为 `telegram-bot = { path = "../telegram-bot" }`。
57. 在 `handlers/src/lib.rs`、`handlers/src/persistence_handler.rs`、`handlers/src/logging_auth.rs` 中：`dbot_core::` → `telegram_bot::`（或按需只改 use，如 `use telegram_bot::{Handler, HandlerResponse, Message, Result};`）。
58. 在 `handlers/src/test/persistence_handler_test.rs`、`handlers/src/test/logging_auth_handler_test.rs` 中：`dbot_core::` → `telegram_bot::`（包括类型与 `Result`、`DbotError`、`HandlerError` 等）。
59. `cargo build -p handlers` 与 `cargo test -p handlers`。

### 6.2 llm-handlers

60. 打开 `llm-handlers/Cargo.toml`：`dbot-core = { path = "../dbot-core" }` 改为 `telegram-bot = { path = "../telegram-bot" }`（若有两条，dev 与 normal 都改）。
61. 在 `llm-handlers/src/sync_llm_handler.rs`、`llm-handlers/src/llm_mention_detector.rs` 中：`dbot_core::` → `telegram_bot::`。
62. 在 `llm-handlers/tests/sync_llm_handler_test.rs` 中：`dbot_core::` → `telegram_bot::`。
63. `cargo build -p llm-handlers` 与 `cargo test -p llm-handlers`。

### 6.3 dbot-llm-cli

64. 打开 `dbot-llm-cli/Cargo.toml`：删除 `dbot-core`、`dbot-telegram` 的 path 依赖；确保仅保留 `telegram-bot = { path = "../telegram-bot" }`（若已有）。
65. 在 `dbot-llm-cli/src/lib.rs` 中：`dbot_telegram::TelegramBotAdapter` → `telegram_bot::TelegramBotAdapter`；`dbot_core::Handler` → `telegram_bot::Handler`；`dbot_core::Bot` → `telegram_bot::Bot`。
66. `cargo build -p dbot-llm-cli`。

### 6.4 dbot-cli

67. 打开 `dbot-cli/Cargo.toml`：若存在对 `dbot-core` 或 `telegram_bot` 的依赖，保留 `telegram_bot` 用于 `BotConfig` 等；删除对 `dbot-core` 的单独依赖。
68. 若 `dbot-cli` 源码中有 `use dbot_core::*`，改为 `use telegram_bot::*` 或按需从 telegram_bot 导入。
69. `cargo build -p dbot-cli`。

### 6.5 crates/memory/memory-handler

70. 打开 `crates/memory/memory-handler/Cargo.toml`：`dbot-core = { path = "../../../dbot-core" }` 改为 `telegram-bot = { path = "../../../telegram-bot" }`（两处：dependencies 与 dev-dependencies）。
71. 在 `memory-handler/src/lib.rs`、`memory-handler/src/memory_handler_test.rs` 中：`dbot_core::` → `telegram_bot::`；若有 `dbot_core::MessageDirection` 等，一并改为 `telegram_bot::`。
72. `cargo build -p memory-handler` 与 `cargo test -p memory-handler`。

### 6.6 crates/llm/telegram-bot-llm

73. 打开 `crates/llm/telegram-bot-llm/Cargo.toml`：`dbot-core` → `telegram-bot`（path 指向 `../../../telegram-bot`）。
74. 在 `telegram-bot-llm/src/main.rs` 中：`dbot_core::init_tracing` → `telegram_bot::init_tracing`。
75. `cargo build -p telegram-bot-llm`。

### 6.7 其他引用（middleware、crates/middleware）

76. 若项目仍使用根目录 `middleware/` 或 `crates/middleware/`：在对应 `Cargo.toml` 中将 `dbot-core` 改为 `telegram-bot`；在 `.rs` 文件中将 `dbot_core::` 全部替换为 `telegram_bot::`。
77. `cargo build --workspace`（若 middleware 在 workspace members 中）。

### 6.8 telegram-bot 自身测试与集成测试

78. 打开 `telegram-bot/tests/runner_integration_test.rs`：`dbot_core::` → `telegram_bot::`。
79. 若存在其他 `telegram-bot/tests/*.rs` 或 `telegram-bot/src/**/test*.rs`，同样替换 `dbot_core`/`handler_chain`/`dbot_telegram` 为 `telegram_bot`。
80. `cargo test -p telegram-bot`。

---

## 阶段七：Workspace 移除三 crate 并收尾

### 7.1 从 workspace 移除成员

81. 打开仓库根目录 `Cargo.toml`，在 `[workspace] members` 中删除 `"dbot-core"`、`"handler-chain"`、`"dbot-telegram"` 三项。
82. 保存后执行 `cargo build --workspace`，应全部通过。

### 7.2 全局检查与测试

83. 在仓库根执行 `cargo build --workspace`，确认无遗漏的 `dbot-core`、`handler-chain`、`dbot-telegram` 依赖。
84. 执行 `cargo test --workspace`，修复仍引用旧 crate 的测试（若有）。
85. 可选：对 `dbot-core`、`handler-chain`、`dbot-telegram` 目录执行 `rg "dbot-core|handler-chain|dbot-telegram" --type rust`，确认无残留引用（除文档或注释外）。

### 7.3 文档与清理（可选）

86. 更新项目 README 或架构说明：说明「框架与应用」现由单一 crate `telegram-bot` 提供，原 dbot-core / handler-chain / dbot-telegram 已合并入内。
87. 若确定不再需要旧代码：可将 `dbot-core/`、`handler-chain/`、`dbot-telegram/` 目录删除或移至 `archive/`；建议在删除前打 tag 或分支备份。

---

## 步骤索引（按文件/操作）

| 步骤 | 操作概要 |
|------|----------|
| 1–4   | 建 core/，迁入 error，mod 与临时 lib 修改 |
| 5–8   | 迁入 types，改 crate 引用 |
| 9–12  | 迁入 bot，改 crate 引用 |
| 13–16 | 迁入 logger |
| 17–18 | 整理 core 导出 |
| 19–22 | 迁入 chain，改 use |
| 23–24 | （可选）迁移 handler_chain 测试 |
| 25–29 | 建 telegram/，迁入 adapters |
| 30–33 | 迁入 bot_adapter |
| 34–39 | 迁入 config、runner，lib 加 mod telegram |
| 40–41 | components 用 crate::core / crate::chain |
| 42–45 | runner、telegram_impl 用内部模块 |
| 46–50 | lib.rs 统一 API；确认 build_handler_chain |
| 51–55 | Cargo.toml 去三 crate 依赖，补依赖，dev-deps |
| 56–59 | handlers 依赖与 use 替换 |
| 60–63 | llm-handlers 依赖与 use 替换 |
| 64–66 | dbot-llm-cli 依赖与 use 替换 |
| 67–69 | dbot-cli 依赖与 use 替换 |
| 70–72 | memory-handler 依赖与 use 替换 |
| 73–75 | telegram-bot-llm 依赖与 use 替换 |
| 76–77 | middleware 等其余引用 |
| 78–80 | telegram-bot 测试与集成测试 |
| 81–82 | workspace 移除三成员，全量 build |
| 83–85 | 全量 test，全局搜索残留 |
| 86–87 | 文档更新与旧目录清理（可选） |

完成以上步骤后，四个 crate 即合并为单一的 **telegram-bot** crate，对外通过 `telegram_bot::*` 使用。
