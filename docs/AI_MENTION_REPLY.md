# Bot 被 @ 时 AI 回复方案

**当前实现**：runner 使用 **SyncAIHandler**（链内同步执行 AI，返回 `Reply` 供 MemoryMiddleware 在 `after()` 存记忆）。历史方案中的 AIQueryHandler/通道已移除；代码以 `ai-handlers/src/sync_ai_handler.rs` 与 `telegram-bot/src/runner.rs` 为准。

## 开发阶段概要

| 阶段 | 内容 | 可选 |
|------|------|------|
| 一 | 环境变量（BOT_TOKEN、OPENAI_API_KEY 等）、依赖 | 否 |
| 二 | HandlerChain 机制（现为 handler-chain crate） | 否 |
| 三 | AI 检测与处理（SyncAIHandler、@ 提及与回复机器人触发） | 否 |
| 四 | 流式响应（AI_USE_STREAMING、先发再编辑） | 是 |
| 五 | 测试与优化 | 否 |
| 六 | 速率限制（RATE_LIMIT_*） | 是 |

## 关键文件

| 文件 | 说明 |
|------|------|
| `ai-handlers/src/sync_ai_handler.rs` | 同步 AI 处理器，链内执行 |
| `ai-handlers/src/ai_mention_detector.rs` | @ 提及与回复检测 |
| `telegram-bot/src/runner.rs` | 组装链、MemoryMiddleware、SyncAIHandler |
| `.env` | BOT_TOKEN、OPENAI_API_KEY、AI_USE_STREAMING、MEMORY_* 等 |

## 数据流（简化）

用户消息 → HandlerChain（AIDetection/回复检测 → SyncAIHandler 构建上下文、调 LLM、发/编辑消息；MemoryMiddleware 存记忆）→ 回复落库。

## 快速开始

```bash
cp .env.example .env   # 填入 BOT_TOKEN、OPENAI_API_KEY
cargo run --package dbot-cli
# Telegram 中 @ 提及 bot 测试
# 可选：AI_USE_STREAMING=true 启用流式
```

## 注意事项

- AI 查询在链内同步执行，不阻塞其他 handler。
- 勿将 BOT_TOKEN、API Key 提交仓库。
- 建议配置速率限制以控制 API 成本；流式通过编辑单条消息更新，不连发多条。

详细实施步骤与历史阶段清单见 git 历史版本；当前行为以代码为准。
