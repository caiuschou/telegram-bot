# Crate 拆分方案：Bot 与 AI 分离

## 1. 目标

- 明确区分 **Bot 层**（传输、生命周期、消息链）与 **AI 层**（LLM 调用、上下文、RAG）。
- 降低耦合：AI 相关 crate 不依赖 Telegram；Bot 层依赖 AI 抽象而非具体实现。
- 便于测试与替换：可单独测试 AI 逻辑或替换 LLM/传输实现。

## 2. 现状概览

### 2.1 当前 Crate 与职责

| Crate | 职责 | 主要依赖 |
|-------|------|----------|
| **dbot-core** | 核心类型（Message、User、Chat）、Handler/Middleware 抽象、Bot trait、日志 | - |
| **telegram-bot** | 主应用：配置、runner、组件组装、handler 链（持久化 + 记忆 + AI handler） | dbot-core, ai-handlers, **telegram-bot-ai**, storage, memory*, middleware, handler-chain, openai-client, embedding |
| **telegram-bot-ai** | `TelegramBotAI`：OpenAI 调用、@mention 解析、`handle_message`（Teloxide Bot + Message） | dbot-core, openai-client, prompt, **teloxide** |
| **ai-handlers** | `SyncAIHandler`（RAG + 调 LLM + 发消息）、`AIDetectionHandler` | dbot-core, storage, **telegram-bot-ai**, memory, prompt, embedding, **teloxide** |

### 2.2 存在的问题

1. **依赖方向混乱**  
   - `telegram-bot` → `ai-handlers` → `telegram-bot-ai`，且 `telegram-bot` 直接依赖 `telegram-bot-ai`。  
   - AI 能力与「Telegram + @mention」绑在同一个 crate（telegram-bot-ai）里。

2. **telegram-bot-ai 职责混合**  
   - 纯 AI：`get_ai_response_with_messages`、流式完成（只依赖 openai-client、prompt）。  
   - Telegram 相关：`handle_message(bot, msg)`、`is_bot_mentioned`、`extract_question`。  
   导致「纯 LLM 调用」无法在不依赖 Teloxide 的前提下被复用（例如 CLI、其他前端）。

3. **ai-handlers 强依赖 Teloxide**  
   - `SyncAIHandler` 直接使用 `teloxide::Bot`、`ChatId` 和 `edit_message_text`。  
   - 不利于在非 Telegram 场景复用同一套 RAG + 回复逻辑。

4. **命名与边界不清晰**  
   - 「bot」与「ai」在 crate 名和模块边界上未形成统一理解。

---

## 3. 目标架构：Bot 与 AI 分层

### 3.1 分层原则

- **Bot 层**：消息接收、路由、链式处理、发送。只依赖「发消息」的抽象（如 `dbot_core::Bot`），不依赖具体 LLM 或 Telegram 实现细节。
- **AI 层**：上下文构建、调用 LLM、流式/非流式。只依赖「发消息」的 trait 和「LLM 客户端」trait，不依赖 Teloxide 类型。

### 3.2 目标 Crate 拓扑（简图）

```
                    ┌─────────────────┐
                    │   dbot-core      │  核心类型、Handler/Middleware、Bot trait
                    └────────┬────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
┌────────────────┐  ┌───────────────┐  ┌────────────────────┐
│ handler-chain  │  │  ai-client    │  │  telegram-bot       │
│ middleware     │  │  (新)         │  │  (仅 Bot 适配与组装) │
└────────────────┘  │  LlmClient    │  └──────────┬─────────┘
                    │  + OpenAI 实现│             │
                    └───────┬───────┘             │
                            │                     │
                            ▼                     │
                    ┌───────────────┐             │
                    │  ai-handlers  │◄────────────┘
                    │  SyncAIHandler│  依赖 Bot trait + LlmClient
                    │  等            │  不依赖 teloxide / telegram-bot-ai
                    └───────────────┘
                            │
                    ┌───────┴───────┐
                    │ memory,       │
                    │ prompt,       │
                    │ storage,      │
                    │ embedding     │
                    └───────────────┘

独立可执行：
┌────────────────────┐     ┌───────────────┐
│ telegram-bot-ai    │────►│  ai-client    │  简单 REPL AI 机器人（无 RAG）
│ (精简为 REPL 入口) │     └───────────────┘
└────────────────────┘
```

---

## 4. 具体拆分方案

### 4.1 新增：`ai-client`（或 `dbot-ai`）

**职责**：纯 LLM 调用抽象与默认实现，与传输、Telegram 完全解耦。

| 项目 | 内容 |
|------|------|
| **位置** | 新 crate：`crates/ai-client/` 或根下 `ai-client/`（与现有 `openai-client` 并列） |
| **对外 API** | **trait `LlmClient`**：`get_ai_response_with_messages`、`get_ai_response_stream_with_messages`（与当前 `TelegramBotAI` 中对应方法签名一致）。 |
| **默认实现** | `OpenAILlmClient`：内部用 `openai-client` + `prompt`，负责 system 消息前置、`ChatMessage` → OpenAI 消息、模型与 base_url 配置。 |
| **依赖** | `openai-client`、`prompt`。**不依赖** `dbot-core`、`teloxide`、`memory`。 |

**从 telegram-bot-ai 迁移的逻辑**（去掉 @mention、Bot、Message）：

- `chat_message_to_openai`
- `get_ai_response_with_messages` / `get_ai_response_stream_with_messages`
- system prompt 的默认内容与注入方式（构造函数 / builder）

**建议的 trait 草图**（与现有行为对齐）：

```rust
// ai-client/src/lib.rs
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn get_ai_response_with_messages(&self, messages: Vec<ChatMessage>) -> Result<String>;
    async fn get_ai_response_stream_with_messages<F, Fut>(&self, messages: Vec<ChatMessage>, callback: F) -> Result<String>
    where
        F: FnMut(StreamChunk) -> Fut,
        Fut: Future<Output = Result<()>>;
}
```

---

### 4.2 重构：`ai-handlers`

**目标**：只依赖「核心抽象」，不依赖 Teloxide 或 `telegram-bot-ai`。

| 项目 | 内容 |
|------|------|
| **依赖** | `dbot-core`、**`ai-client`**、`memory`、`prompt`、`storage`、`embedding`。**移除**：`telegram-bot-ai`、`teloxide`。 |
| **SyncAIHandler** | 持有 `Arc<dyn LlmClient>` 与 `Arc<dyn dbot_core::Bot>`，不再持有 `TelegramBotAI` 或 `teloxide::Bot`。 |
| **发消息** | 使用 `dbot_core::Bot::send_message(chat, text)`，参数来自 `message.chat`。 |
| **流式编辑** | 需要「编辑已发消息」能力：在 **dbot-core** 的 `Bot` trait 中新增 `edit_message(chat, message_id, text)`（可选或带默认实现），由 **telegram-bot** 的 Bot 实现体实现为 `edit_message_text`。 |

**接口变化要点**：

- 构造：`SyncAIHandler::new(..., llm_client: Arc<dyn LlmClient>, bot: Arc<dyn dbot_core::Bot>, ...)`。
- 内部：所有 `self.bot.send_message(chat_id, ...)` 改为 `self.bot.send_message(&message.chat, ...)`；流式处改为 `self.bot.edit_message(&message.chat, message_id, ...)`（具体类型由 dbot-core 的 Message 或 Bot 定义决定）。

---

### 4.3 扩展：`dbot-core` 的 Bot trait

为支持流式回复的「先发一条再编辑」：

- 在 `Bot` trait 中增加：  
  `async fn edit_message(&self, chat: &Chat, message_id: &str, text: &str) -> Result<();`  
  - 若希望非流式场景不强制实现，可设计为：带默认实现返回「未实现」错误，或单独 trait（如 `BotWithEdit`）由 telegram 实现。
- **telegram-bot** 中实现该方法的实现体内部调用 `teloxide::Bot::edit_message_text`（需要当前 API 下 message_id 的表示方式与 Telegram 一致）。

这样 **ai-handlers** 只依赖 `dbot_core::Bot`，不再依赖 `teloxide`。

---

### 4.4 重构：`telegram-bot`

**目标**：作为「Bot 层」的组装入口，只依赖 AI 的抽象（LlmClient），不直接依赖 telegram-bot-ai。

| 项目 | 内容 |
|------|------|
| **依赖** | 保留：dbot-core, handler-chain, middleware, storage, memory*, embedding 等。**改为**：依赖 **ai-client** 和 **ai-handlers**；**移除** 对 **telegram-bot-ai** 的依赖。 |
| **组件构建** | 在 `build_bot_components` / `initialize_bot_components` 中：构造 `OpenAILlmClient`（或其它 `LlmClient` 实现），以及实现 `dbot_core::Bot` 的 Teloxide 适配器（现有 `telegram_impl` 中的类型），将 `Arc<dyn LlmClient>` 与 `Arc<dyn Bot>` 传入 `SyncAIHandler::new`。 |
| **配置** | 原用于 `TelegramBotAI` 的配置（模型、system prompt、base_url 等）改为用于构建 `OpenAILlmClient`。 |

这样 **telegram-bot** 在架构上明确为「Bot + 链 + AI handler」，AI 能力通过 **ai-client** + **ai-handlers** 注入。

---

### 4.5 精简：`telegram-bot-ai`

**目标**：仅作为「无 RAG、仅 @mention」的独立可执行示例/二进制，依赖 **ai-client**，不再被其它 crate 依赖。

| 项目 | 内容 |
|------|------|
| **职责** | 提供基于 REPL 的简单 AI 机器人：收消息 → 检测 @mention → 调 LLM → 回发。不负责 RAG、记忆、持久化。 |
| **依赖** | **ai-client**（LlmClient）、**teloxide**、**dbot-core**（仅 init_tracing 等可选）。**移除** 对 **openai-client** / **prompt** 的直接依赖（通过 ai-client 间接使用）。 |
| **实现** | `TelegramBotAI` 保留为薄封装：持有 `Arc<dyn LlmClient>` + `bot_username`，实现 `handle_message(bot, msg)`（内部调用 `llm_client.get_ai_response(...)` 或 stream，并用 `bot.send_message` 等）。原有「直接调 OpenAI」的逻辑迁移到 ai-client。 |
| **二进制** | 现有 `main.rs` 保持不变：启动 REPL，用 `TelegramBotAI` 处理消息。 |

可选：若希望与其它示例统一，也可将 `telegram-bot-ai` 的二进制迁到 `telegram-bot-examples`（例如 `ai_repl.rs`），保留一个小 crate 仅做薄封装；两种方式二选一即可。

---

## 5. 拆分后 Crate 一览

| Crate | 归属 | 职责摘要 |
|-------|------|----------|
| **dbot-core** | Bot | 类型、Handler/Middleware、Bot trait（含可选 edit_message） |
| **handler-chain** | Bot | 链式执行 |
| **middleware** | Bot | 持久化、记忆等中间件 |
| **telegram-bot** | Bot | 配置、runner、Teloxide 适配、组装链与 SyncAIHandler |
| **ai-client** | AI | LlmClient trait + OpenAI 实现 |
| **ai-handlers** | AI | SyncAIHandler、AIDetectionHandler（RAG + LlmClient + Bot trait） |
| **telegram-bot-ai** | 示例/独立二进制 | 简单 REPL AI 机器人，依赖 ai-client |
| **memory** / **prompt** / **storage** / **embedding** / **openai-client** | 共享 | 保持现状，被 Bot 与 AI 层按需使用 |

---

## 6. 实施顺序建议

| 步骤 | 内容 | 说明 |
|------|------|------|
| 1 | 新增 **ai-client**，从 telegram-bot-ai 迁出 LLM 调用逻辑，实现 `LlmClient` + `OpenAILlmClient` | 可先保留 telegram-bot-ai 内部调用旧实现，再切到 ai-client |
| 2 | 在 **dbot-core** 的 `Bot` trait 中增加 `edit_message`（或 `BotWithEdit`） | 便于 ai-handlers 去掉对 teloxide 的依赖 |
| 3 | **telegram-bot** 中为现有 Bot 实现体实现 `edit_message`（委托给 teloxide） | 与步骤 2 同步或紧随其后 |
| 4 | **ai-handlers** 重构：改为依赖 `LlmClient` + `dbot_core::Bot`，移除对 telegram-bot-ai、teloxide 的依赖 | 构造处改为注入 Arc<dyn LlmClient> 与 Arc<dyn Bot> |
| 5 | **telegram-bot** 组装处改为构建 `OpenAILlmClient` 并注入 ai-handlers，移除对 telegram-bot-ai 的依赖 | 更新 Cargo.toml 与 runner/组件构建代码 |
| 6 | **telegram-bot-ai** 重构为仅依赖 ai-client，内部使用 `LlmClient`，保留 REPL 与 @mention 逻辑 | 可选：二进制迁到 telegram-bot-examples |
| 7 | 全量测试：telegram-bot（含 RAG）、telegram-bot-ai 独立运行；补充/更新单元测试与文档 | 按 AGENTS 要求更新 CHANGELOGS.md |

---

## 7. 风险与注意点

- **Bot trait 的 edit_message**：需约定 `message_id` 在 core 中的表示（例如字符串），与 Telegram 的 message id 一致，以便 telegram-bot 实现时直接转发。
- **向后兼容**：若存在外部依赖 `telegram-bot-ai` 或直接依赖 `TelegramBotAI` 的代码，需要在 CHANGELOG 中说明迁移路径（改用 ai-client + 自建 REPL 或使用新的 telegram-bot-ai 薄封装）。
- **配置**：原 `telegram-bot` 中与 AI 相关的 env（如 `AI_MODEL`、`AI_SYSTEM_PROMPT`）改为用于构建 `OpenAILlmClient`，需在文档和 .env.example 中说明归属。

---

## 8. 文档与规范

- 在 **CHANGELOGS.md** 中增加「Crate 拆分：Bot 与 AI 分离」条目，按步骤简述变更。
- 更新 **README** / **ARCHITECTURE** 类文档中的架构图与依赖说明，使其与上述拓扑一致。
- 各 crate 的 README 或 doc 中注明其属于「Bot 层」还是「AI 层」，以及主要依赖与使用场景。

以上方案在不动现有功能的前提下，通过新增 **ai-client** 和调整依赖边界，清晰区分 Bot 与 AI，并便于后续扩展（如多 LLM、多传输前端）。

---

## 9. 相关文档

- [Crate 与文件索引](../CRATES.md)：每个 crate 及对应文件与描述，遵循「每个 crate 尽量简单」。
- [开发计划](DEVELOPMENT_PLAN.md)：按阶段与任务表执行的详细开发计划（P0～P3、验收与测试）。
- [Telegram Bot 框架设计](telegram-bot-framework-design.md)：从本库抽离「专门处理 Telegram Bot 的框架」、简化开发的方案（框架只做 Telegram 接入 + 链式处理，AI/持久化/记忆作为插件）。
