# Crate 拆分：Bot 与 AI 分离

## 目标

- 区分 **Bot 层**（传输、消息链）与 **AI 层**（LLM、RAG）；降低耦合。
- AI 相关 crate 不依赖 Telegram；Bot 层依赖 AI 抽象（LlmClient、Bot trait）。
- 便于测试与替换（mock LlmClient/Bot、换传输实现）。

## 目标架构摘要

| 层 | Crate | 职责 |
|------|------|------|
| 核心 | dbot-core | 类型、Handler/Middleware/Bot trait |
| 框架 | dbot-telegram | Telegram 适配、run_repl、最小配置 |
| AI 抽象 | ai-client | LlmClient、OpenAILlmClient |
| AI 处理 | ai-handlers | SyncAIHandler（RAG + LlmClient + Bot） |
| 主应用 | telegram-bot | 配置、组装链、run_bot |
| REPL 示例 | telegram-bot-ai | 依赖 ai-client 的独立 REPL |

## 实施顺序

1. dbot-telegram（框架抽离）；2. ai-client + Bot.edit_message；3. ai-handlers 重构（持有 LlmClient + Bot）；4. telegram-bot 注入、telegram-bot-ai 精简。

当前已完成；详细任务表见 [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md)。
