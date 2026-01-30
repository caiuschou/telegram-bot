# 开发计划：框架抽离与 Bot/AI 拆分

## 目标与范围

| 目标 | 说明 |
|------|------|
| 框架抽离 | 新增 dbot-telegram：仅 Telegram 接入 + 消息链 + 最小配置；telegram-bot 改为依赖框架并组装链与业务配置 |
| Bot/AI 拆分 | 新增 ai-client（LlmClient + OpenAILlmClient）；ai-handlers 只依赖 Bot + LlmClient；telegram-bot-ai 精简为依赖 ai-client 的 REPL |
| 原则 | 每个 crate 尽量简单；见 [CRATES.md](../CRATES.md) |

## 阶段总览

| 阶段 | 名称 | 目标 | 前置 |
|------|------|------|------|
| P0 | 框架层（dbot-telegram） | 抽离 Telegram 接入与 run_repl，最小配置 | 无 |
| P1 | AI 抽象层（ai-client + Bot.edit_message） | LlmClient trait、OpenAILlmClient；Bot 支持 edit_message | 无 |
| P2 | 应用与 Handler 重构 | telegram-bot 用框架 + 注入 LlmClient/Bot；ai-handlers 去 teloxide | P0、P1 |
| P3 | 测试与文档 | 全量测试、CHANGELOG、CRATES/README 更新 | P2 |

## 详细任务

- **P0**：新建 dbot-telegram、迁出适配器与 Bot 实现、TelegramConfig、run_repl；telegram-bot 改为依赖框架；更新 CRATES/README。
- **P1**：新建 ai-client、LlmClient trait、OpenAILlmClient；dbot-core Bot 增加 edit_message；dbot-telegram 实现 edit_message。
- **P2**：ai-handlers 持有 Arc<LlmClient>、Arc<dyn Bot>；telegram-bot runner 注入；telegram-bot-ai 用 ai-client。
- **P3**：测试通过、文档与索引更新。

当前状态：P0–P2 已完成；具体任务表与验收见 git 历史。相关方案见 [crate-split-bot-ai-plan.md](crate-split-bot-ai-plan.md)、[telegram-bot-framework-design.md](telegram-bot-framework-design.md)。
