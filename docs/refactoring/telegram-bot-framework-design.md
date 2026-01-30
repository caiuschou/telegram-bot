# Telegram Bot 框架设计（从本库抽离）

## 目标

- 框架只做 **Telegram 接入 + 消息链 + 扩展点**，不捆绑 AI/持久化/记忆。
- 最少配置（token、可选 api_url、log_file）即可跑 REPL。
- 通过 Handler/Middleware 扩展；AI、持久化、记忆以「插件」形式接入。

## 框架做什么（In Scope）

| 能力 | 说明 |
|------|------|
| Telegram 接入 | teloxide long polling（REPL），token / 可选 API URL |
| 消息标准化 | teloxide Message → dbot_core::Message |
| 处理链 | Middleware.before → Handler 列表 → Middleware.after |
| 发送抽象 | dbot_core::Bot trait（send_message、reply_to、edit_message） |
| 运行入口 | run_repl(bot, handler_chain, bot_username) |

## 框架不做什么（Out of Scope）

- 持久化、记忆/RAG、AI 调用、业务配置 → 由应用或 middleware/ai-handlers 等提供。

实现见 crates/dbot-telegram；应用组装见 telegram-bot/runner。详细 API 与抽离步骤见 [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md)。
