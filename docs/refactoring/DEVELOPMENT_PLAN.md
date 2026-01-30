# 开发计划：框架抽离与 Bot/AI 拆分

本文档给出从当前库抽离 **Telegram Bot 框架（dbot-telegram）** 并完成 **Bot 与 AI 分层（ai-client、ai-handlers 解耦）** 的详细开发计划，以表格为主，便于按任务执行与跟踪。

---

## 1. 目标与范围

| 目标 | 说明 |
|------|------|
| **框架抽离** | 新增 `dbot-telegram`，只做 Telegram 接入 + 消息链 + 最小配置；telegram-bot 主应用改为依赖框架并只做「组装链 + 业务配置」。 |
| **Bot/AI 拆分** | 新增 `ai-client`（LlmClient + OpenAI 实现）；ai-handlers 只依赖 `dbot_core::Bot` + `LlmClient`，不依赖 teloxide/telegram-bot-ai；telegram-bot-ai 精简为依赖 ai-client 的 REPL 示例。 |
| **每个 crate 尽量简单** | 单一职责、少依赖、文件与描述见 [CRATES.md](../CRATES.md)。 |

---

## 2. 阶段总览

| 阶段 | 名称 | 目标 | 交付物 | 前置依赖 |
|------|------|------|--------|----------|
| **P0** | 框架层（dbot-telegram） | 抽离 Telegram 接入与链式运行，最小配置 | 新 crate dbot-telegram；telegram-bot 改为使用框架入口 | 无 |
| **P1** | AI 抽象层（ai-client + Bot.edit_message） | 独立 LLM 调用抽象；Bot 支持编辑消息 | 新 crate ai-client；dbot-core Bot 扩展 | 无（可与 P0 并行） |
| **P2** | 应用与 Handler 重构 | telegram-bot 用框架 + 注入 LlmClient/Bot；ai-handlers 去 teloxide | 重构 telegram-bot、ai-handlers；telegram-bot-ai 用 ai-client | P0、P1 |
| **P3** | 测试与文档 | 全量测试、CHANGELOG、CRATES/README 更新 | 通过测试、文档与索引更新 | P2 |

---

## 3. 详细任务表（按阶段）

### 3.1 阶段 P0：框架层（dbot-telegram）

| 任务 ID | 任务 | 涉及 Crate/文件 | 验收标准 | 测试/文档 |
|---------|------|-----------------|----------|-----------|
| P0-1 | 新建 crate `dbot-telegram`，Cargo.toml 仅依赖 dbot-core、handler-chain、teloxide、tracing、anyhow | `crates/dbot-telegram/Cargo.toml` | workspace 能 build；无 storage/memory/ai-handlers 依赖 | - |
| P0-2 | 从 telegram-bot 迁出适配器：teloxide Message → core::Message，实现 ToCoreMessage/ToCoreUser | `dbot-telegram/src/adapters.rs` | 与当前 telegram-bot/adapters 行为一致，单元测试覆盖主要字段 | `dbot-telegram/tests/` 或 crate 内 `#[cfg(test)]` |
| P0-3 | 从 telegram-bot 迁出 Bot trait 实现（发送消息），包装 teloxide::Bot | `dbot-telegram/src/bot_adapter.rs` 或 `telegram_impl.rs` | 实现 dbot_core::Bot；send_message、reply_to 行为与现有一致 | 单元测试或集成测试 |
| P0-4 | 定义 TelegramConfig：仅 token、api_url、log_file；提供 from_env() | `dbot-telegram/src/config.rs` | 从 BOT_TOKEN、TELEGRAM_API_URL、可选 LOG_FILE 加载 | 单元测试 |
| P0-5 | 实现 TelegramRunner：new(token)、api_url()、log_file()、handler_chain()、run().await | `dbot-telegram/src/runner.rs` | REPL 启动；每条消息 to_core → chain.handle(core_msg)；可选 get_me 缓存 bot_username | 与 telegram-bot 集成测试对比行为 |
| P0-6 | dbot-telegram 对外 API：pub use config、runner、adapters、bot_adapter | `dbot-telegram/src/lib.rs` | 用户可 TelegramRunner::new(...).handler_chain(chain).run().await | - |
| P0-7 | telegram-bot 改为依赖 dbot-telegram；runner 内用 TelegramRunner + 现有 build 链逻辑，移除本地 REPL/适配器重复代码 | `telegram-bot/Cargo.toml`，`telegram-bot/src/runner.rs`，删除或精简 `adapters.rs`/`telegram_impl.rs`（改为 re-export 或删除） | telegram-bot 主应用仍能正常跑，行为与当前一致 | 现有 telegram-bot 集成测试通过 |
| P0-8 | 更新 CRATES.md、README 项目结构：增加 dbot-telegram，telegram-bot 描述改为「使用框架 + 组装链」 | `docs/CRATES.md`，`README.md` | 索引与结构图反映新 crate 与职责 | - |

### 3.2 阶段 P1：AI 抽象层（ai-client + Bot.edit_message）

| 任务 ID | 任务 | 涉及 Crate/文件 | 验收标准 | 测试/文档 |
|---------|------|----------------|----------|-----------|
| P1-1 | 新建 crate `ai-client`，Cargo.toml 仅依赖 openai-client、prompt | `ai-client/Cargo.toml` 或 `crates/ai-client/Cargo.toml` | workspace 能 build；无 dbot-core、teloxide | - |
| P1-2 | 定义 LlmClient trait：get_ai_response_with_messages、get_ai_response_stream_with_messages（签名与当前 TelegramBotAI 对齐） | `ai-client/src/lib.rs` | 与 telegram-bot-ai 现有调用方式兼容，便于后续替换 | - |
| P1-3 | 从 telegram-bot-ai 迁出 LLM 调用逻辑：ChatMessage→OpenAI 消息、system 前置、chat_completion/stream | `ai-client/src/openai_client.rs` 或 `lib.rs` 内 OpenAILlmClient | 行为与 TelegramBotAI 的 get_ai_response_with_messages / stream 一致 | 单元测试：给定 messages 出相同请求/响应形状 |
| P1-4 | OpenAILlmClient 构造：api_key、base_url、model、system_prompt 可选 | `ai-client` | 与现有 BotConfig 中 AI 相关字段可对应 | - |
| P1-5 | 在 dbot-core 的 Bot trait 中增加 edit_message(chat, message_id, text) | `dbot-core/src/bot.rs`（及 types 若需 MessageId） | 流式回复「先发一条再编辑」可由 Handler 调用 | 若已有 Bot 实现，需同步实现该方法的默认或具体实现 |
| P1-6 | dbot-telegram 的 Bot 实现体实现 edit_message，委托 teloxide edit_message_text | `dbot-telegram/src/bot_adapter.rs` | Telegram 流式编辑行为与当前一致 | 集成或手动验证 |
| P1-7 | ai-client 对外 API：LlmClient、OpenAILlmClient、StreamChunk 等 | `ai-client/src/lib.rs` | 供 ai-handlers、telegram-bot-ai 使用 | - |
| P1-8 | 更新 CRATES.md：新增 ai-client 条目及文件描述 | `docs/CRATES.md` | 索引含 ai-client 与各文件说明 | - |

### 3.3 阶段 P2：应用与 Handler 重构

| 任务 ID | 任务 | 涉及 Crate/文件 | 验收标准 | 测试/文档 |
|---------|------|----------------|----------|-----------|
| P2-1 | ai-handlers：SyncAIHandler 改为持有 Arc<dyn LlmClient>、Arc<dyn dbot_core::Bot>，不再持有 TelegramBotAI、teloxide::Bot | `ai-handlers/src/sync_ai_handler.rs`，`ai-handlers/Cargo.toml` | 移除对 telegram-bot-ai、teloxide 的依赖；构造处接收 llm_client + bot | 现有 sync_ai_handler 测试通过（需注入 mock LlmClient/Bot） |
| P2-2 | ai-handlers：发消息统一用 Bot::send_message(&message.chat, text)；流式用 Bot::edit_message(chat, message_id, text) | `ai-handlers/src/sync_ai_handler.rs` | 无 teloxide 类型；ChatId 等用 core::Chat、message_id 用字符串 | 单元/集成测试 |
| P2-3 | telegram-bot：build_bot_components 中构造 OpenAILlmClient（用 config 的 AI 相关字段），以及 dbot-telegram 的 Bot 实现体；将 Arc<dyn LlmClient>、Arc<dyn Bot> 传入 SyncAIHandler::new | `telegram-bot/src/runner.rs` | 主应用仍能跑 RAG + 流式/非流式；无 TelegramBotAI 依赖 | 集成测试 |
| P2-4 | telegram-bot：Cargo.toml 移除 telegram-bot-ai 依赖，增加 ai-client 依赖 | `telegram-bot/Cargo.toml` | 编译通过，无循环依赖 | - |
| P2-5 | telegram-bot-ai：改为依赖 ai-client；TelegramBotAI 内部持有 Arc<dyn LlmClient> + bot_username，handle_message 调用 llm_client.get_ai_response 等并用 bot.send_message | `telegram-bot-ai/src/lib.rs`，`telegram-bot-ai/Cargo.toml` | 移除对 openai-client、prompt 的直接依赖；行为与当前一致 | telegram-bot-ai main 跑通 |
| P2-6 | telegram-bot-ai：main 保持不变，仅依赖新 TelegramBotAI（内部用 ai-client） | `telegram-bot-ai/src/main.rs` | 独立 REPL AI 机器人可运行 | 手动或自动化验证 |
| P2-7 | 更新 CRATES.md：ai-handlers、telegram-bot、telegram-bot-ai 的依赖与文件描述 | `docs/CRATES.md` | 索引准确反映新依赖关系 | - |

### 3.4 阶段 P3：测试与文档

| 任务 ID | 任务 | 涉及 Crate/文件 | 验收标准 | 测试/文档 |
|---------|------|----------------|----------|-----------|
| P3-1 | 全 workspace cargo build、cargo test | 全部 | 无报错 | - |
| P3-2 | 运行 telegram-bot 主应用：收消息 → RAG → 流式/非流式回复、记忆与持久化正常 | 手动或 CI | 行为与重构前一致 | - |
| P3-3 | 运行 telegram-bot-ai 独立二进制：@mention → LLM 回复 | 手动或 CI | 行为与重构前一致 | - |
| P3-4 | CHANGELOGS.md 增加条目：框架抽离（dbot-telegram）、Bot/AI 拆分（ai-client、ai-handlers 解耦）、telegram-bot-ai 精简 | `CHANGELOGS.md` | 按步骤简述变更与迁移说明 | - |
| P3-5 | README、dbot-core/ARCHITECTURE 等：更新架构图与依赖说明，标明 dbot-telegram、ai-client | `README.md`，`dbot-core/ARCHITECTURE.md` | 与新拓扑一致 | - |
| P3-6 | 各 crate README 或 doc：注明「Bot 层」/「AI 层」及主要使用场景（可选） | 各 crate | 便于后续维护与接入 | - |

---

## 4. 任务依赖与建议顺序

| 顺序 | 任务 ID | 说明 |
|------|---------|------|
| 1 | P0-1～P0-6 | 先完成 dbot-telegram 本体，再改 telegram-bot |
| 2 | P0-7 | telegram-bot 切到框架 |
| 3 | P1-1～P1-4，P1-7 | ai-client 可与 P0 并行开发 |
| 4 | P1-5～P1-6 | Bot.edit_message 需在 ai-handlers 改前落地（P2-2 依赖） |
| 5 | P2-1～P2-2 | ai-handlers 去 teloxide，依赖 P1 完成 |
| 6 | P2-3～P2-4 | telegram-bot 注入 LlmClient/Bot，依赖 P0-7、P2-1 |
| 7 | P2-5～P2-6 | telegram-bot-ai 切到 ai-client |
| 8 | P0-8，P1-8，P2-7，P3-* | 文档与全量测试收尾 |

**可并行**：P0 与 P1（除 P1-5/P1-6 与 P2 的衔接）可大部分并行；P1-5/P1-6 必须在 P2-1 前完成。

---

## 5. 文件变更清单（汇总）

| Crate | 新增 | 修改 | 删除/迁移出 |
|-------|------|------|-------------|
| **dbot-telegram** | 整个 crate（config、runner、adapters、bot_adapter、lib） | - | - |
| **ai-client** | 整个 crate（LlmClient、OpenAILlmClient） | - | - |
| **dbot-core** | - | bot.rs（edit_message） | - |
| **telegram-bot** | - | Cargo.toml、runner.rs；可选保留 adapters/telegram_impl 为 re-export 或删除 | 适配器/REPL 逻辑迁至 dbot-telegram |
| **ai-handlers** | - | sync_ai_handler.rs、Cargo.toml | 对 telegram-bot-ai、teloxide 的依赖 |
| **telegram-bot-ai** | - | lib.rs、Cargo.toml | 对 openai-client、prompt 的直接依赖 |
| **docs** | - | CRATES.md、README.md、CHANGELOGS.md、ARCHITECTURE 等 | - |

---

## 6. 风险与回滚

| 风险 | 缓解 | 回滚 |
|------|------|------|
| 流式编辑 message_id 类型在 core 与 Telegram 不一致 | 统一用字符串，与 Telegram API message_id 一致 | 保留原 SyncAIHandler 分支，feature 或配置切换 |
| 外部依赖 telegram-bot-ai 的 API | CHANGELOG 说明迁移路径；保留 TelegramBotAI 对外签名，内部委托 ai-client | 暂不删 telegram-bot-ai，仅改实现 |
| 框架与主应用行为不一致 | P0-7 后跑现有 telegram-bot 集成测试；P2 后对比主应用与 telegram-bot-ai 行为 | 未通过则回退对应 MR，保留旧 runner/适配器 |

---

## 7. 参考文档

| 文档 | 说明 |
|------|------|
| [Crate 与文件索引](../CRATES.md) | 各 crate 及文件描述，遵循「每个 crate 尽量简单」 |
| [Crate 拆分方案：Bot 与 AI 分离](crate-split-bot-ai-plan.md) | ai-client、ai-handlers 解耦、Bot trait 扩展 |
| [Telegram Bot 框架设计](telegram-bot-framework-design.md) | dbot-telegram 范围、API、从 telegram-bot 抽离方式 |

---

## 8. 修订记录

| 日期 | 变更 |
|------|------|
| 初版 | 新增开发计划：P0～P3 阶段、任务表、依赖顺序、文件变更与风险 |
