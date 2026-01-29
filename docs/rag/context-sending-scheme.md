# Context 发送方案（OpenAI 对话）

本文档约定：**记忆上下文（context）如何组装并以 JSON 消息形式发送到 OpenAI Chat Completions API**。

## 1. 目标

- 明确 context 的**组成**与**文本格式**。
- 明确发送到 API 的 **messages 结构**（system / user 角色与内容）。
- **类型与 API 一一对应**：`prompt::format_for_model_as_messages` 与 `Context::to_messages` 返回 `Vec<ChatMessage>`，每条 `ChatMessage` 对应 OpenAI 请求体中 `messages` 数组的一项 `{ "role", "content" }`。
- 便于后续扩展（如多轮 user、可配置 system）。

## 2. 历史消息如何融入 OpenAI 会话

本节单独说明：**历史对话（近期消息）从存储到最终进入 OpenAI 请求的完整链路**，以及为何是「文本块融入」而非「多条 API 消息」。

### 2.1 历史消息从哪里来

| 步骤 | 位置 | 说明 |
|------|------|------|
| 1. 存储 | `MemoryStore`（如 Lance / SQLite） | 用户与助手的每条消息写入时带 `role`（User/Assistant/System）和 `content`。 |
| 2. 按会话/用户查询 | `RecentMessagesStrategy` | 调用 `store.search_by_conversation(conversation_id)` 或 `store.search_by_user(user_id)`，按时间顺序取最近 N 条，得到 `Vec<MemoryEntry>`。 |
| 3. 单条格式化 | `memory-strategies::utils::format_message` | 每条 `MemoryEntry` 转为 `"{Role}: {content}"`，其中 Role 来自 `entry.metadata.role`：`User` → `"User:"`，`Assistant` → `"Assistant:"`，`System` → `"System:"`。 |
| 4. 归入 Context | `ContextBuilder::build` | 策略返回 `StrategyResult::Messages { category: Recent, messages }`，这些字符串进入 `Context.recent_messages`。 |
| 5. 拼成一段文本 | `prompt::format_for_model` | 输出 `"Conversation (recent):\n"` + 每行一条 `"User: ..."` / `"Assistant: ..."`，即历史消息是**多行纯文本**，不是 JSON 数组。 |
| 6. 放入本次请求 | `SyncAIHandler::build_messages_for_ai` → `Context::to_messages` + `TelegramBotAI::get_ai_response_with_messages` / `get_ai_response_stream_with_messages` | 将 context 转为 `Vec<ChatMessage>`（system / user / assistant），与当前用户问题一起作为多条 API 消息发送。 |

### 2.2 融入方式：多条 API 消息

OpenAI Chat Completions 的 `messages` 支持多轮对话，例如：

```json
[
  { "role": "system", "content": "You are helpful." },
  { "role": "user", "content": "你好" },
  { "role": "assistant", "content": "你好！有什么可以帮你的？" },
  { "role": "user", "content": "今天天气怎么样" }
]
```

**当前实现采用上述方式**。`Context::to_messages(include_system, current_question)` 将近期对话、语义参考、用户偏好与当前问题组装为 `Vec<ChatMessage>`，其中：

- 可选一条 `role: system`（系统说明）；
- 近期对话（recent）被解析为多条 `role: user` / `role: assistant` / `role: system`，与 API 一一对应；
- 用户偏好与语义参考合并为一条或多条 `role: user` 的 content；
- 最后一条 `role: user` 为当前用户问题。

`SyncAIHandler::build_messages_for_ai` 调用 `to_messages` 后，将结果传给 `TelegramBotAI::get_ai_response_with_messages` 或 `get_ai_response_stream_with_messages`，直接作为 OpenAI 请求体中的 `messages` 数组。

### 2.3 单条历史消息的文本格式

每条来自存储的消息在 context 中呈现为一行（或一段）文本：

- `MemoryRole::User` → `"User: {content}"`
- `MemoryRole::Assistant` → `"Assistant: {content}"`
- `MemoryRole::System` → `"System: {content}"`

近期对话整体位于 `Conversation (recent):` 标题之下；语义检索得到的相关历史位于 `Relevant reference (semantic):` 之下，格式相同（也是 `"User: ..."` / `"Assistant: ..."`）。

---

## 3. Context 的组成

Context 由 memory 模块的 `ContextBuilder` 构建，包含：

| 部分 | 来源策略 | 说明 |
|-----|----------|------|
| 近期对话 | `RecentMessagesStrategy` | 当前会话最近 N 条用户/助手消息 |
| 语义参考 | `SemanticSearchStrategy` | 按当前问题向量检索到的相关历史 |
| 用户偏好 | `UserPreferencesStrategy` | 抽取的用户偏好摘要 |
| 系统说明 | `ContextBuilder::with_system_message`（可选） | 行为/人设说明，当前未从 builder 传入 |

构建结果由 `Context::format_for_model(include_system)` 转成**纯文本**，格式由 `prompt` crate 的 `format_for_model` 定义。

## 4. Context 的文本格式（prompt 约定）

`prompt::format_for_model` 输出顺序与格式如下（换行分隔，无 JSON）：

```text
[可选] System: {system_message}

[可选] User Preferences: {user_preferences}

[可选] Conversation (recent):
{recent_message_1}
{recent_message_2}
...

[可选] Relevant reference (semantic):
{semantic_message_1}
...
```

- Section 标题固定：`Conversation (recent):`、`Relevant reference (semantic):`，便于模型区分「当前对话」与「参考引用」。
- 当前实现中，`SyncAIHandler::build_memory_context` 使用 `format_for_model(false)`，即**不**在 context 文本中包含 System 行；系统指令由下游单独作为 API 的 system 消息发送。

## 5. 发送到 OpenAI 的方式（当前方案）

请求体为 **JSON**，`Content-Type: application/json`。  
`messages` 为数组，每项为 `{ "role": "system" | "user" | "assistant", "content": "..." }`。

### 5.1 当前实现：与 OpenAI 一一对应的消息列表

发送到 API 的 `messages` 由 `Vec<ChatMessage>` 转换而来，**每条 `ChatMessage` 对应一条 API 消息**：

| 顺序 | role | content |
|------|------|--------|
| 1 | `system` | 固定系统指令（`TelegramBotAI::DEFAULT_SYSTEM_CONTENT`：中文助手、纯文本、适合 Telegram） |
| 2（可选） | `user` | **context 文本块**（User Preferences + Conversation (recent) + Relevant reference），仅当 context 非空时存在 |
| 最后 | `user` | **当前用户问题** |

- **System**：在 `telegram-bot-ai` 中 prepend，不占 `format_for_model_as_messages` / `to_messages` 返回的列表。
- **User**：由 `Context::to_messages(false, question)` 或 `prompt::format_for_model_as_messages(...)` 得到；无 context 时为 `[User(question)]`，有 context 时为 `[User(context_block), User(question)]`。

即：**context 仍以一段纯文本放在一条 user 的 content 中，但与「当前问题」拆成两条 user 消息**；类型上使用 `MessageRole` / `ChatMessage` 与 OpenAI 的 `role` / `content` 一一对应。

### 5.2 融入后的 JSON 示例（含查询到的历史消息）

**本案例中查询到的历史信息**（各策略返回后、写入 context 前的数据）：

| 来源 | 策略 | 查询到的历史信息（格式化后） |
|------|------|------------------------------|
| 近期对话 | `RecentMessagesStrategy` | ① `User: 狗吃什么`<br>② `Assistant: 狗可以吃狗粮、肉类和部分蔬菜。`<br>③ `User: 猫吃什么`<br>④ `Assistant: 猫是肉食动物，适合吃猫粮、鱼肉和煮熟的鸡肉。` |
| 语义参考 | `SemanticSearchStrategy` | ① `User: 我家的猫喜欢鱼`<br>② `Assistant: 可以适量喂鱼，注意去刺。` |
| 用户偏好 | `UserPreferencesStrategy` | `用户偏好喝茶。` |

当前用户提问：**那猫呢？**

上述历史信息经 `format_for_model` 拼成 context 文本，再与「用户提问: 那猫呢？」拼接后，作为**一条** `user` 的 `content` 发送。请求体中 `messages` 示例如下：

```json
{
  "model": "gpt-3.5-turbo",
  "messages": [
    {
      "role": "system",
      "content": "你是一个有用的助手，用中文回答问题。不要使用Markdown格式，不要使用任何格式化符号（如*、_、`、#等），输出纯文本，适合Telegram消息。"
    },
    {
      "role": "user",
      "content": "User Preferences: 用户偏好喝茶。\n\nConversation (recent):\nUser: 狗吃什么\nAssistant: 狗可以吃狗粮、肉类和部分蔬菜。\nUser: 猫吃什么\nAssistant: 猫是肉食动物，适合吃猫粮、鱼肉和煮熟的鸡肉。\n\nRelevant reference (semantic):\nUser: 我家的猫喜欢鱼\nAssistant: 可以适量喂鱼，注意去刺。\n\n用户提问: 那猫呢？"
    }
  ]
}
```

说明：

- **第 1 条**：`role: "system"`，固定系统指令，与历史无关。
- **第 2 条**：`role: "user"`，`content` 由上表中的**查询到的历史信息**按约定格式拼接而成：
  - `User Preferences: 用户偏好喝茶。` ← 对应上表「用户偏好」一行。
  - `Conversation (recent):` 下 4 行 ← 对应上表「近期对话」①②③④。
  - `Relevant reference (semantic):` 下 2 行 ← 对应上表「语义参考」①②。
  - `用户提问: 那猫呢？` ← 当前轮用户问题。

因此，**查询到的历史消息并非多条 `user`/`assistant` 的 API 消息**，而是全部拼进这一条 `user` 的 `content` 字符串里；上表即「融入前」各策略产出的历史信息，JSON 中即「融入后」的形态。

### 5.3 对应代码位置（含「一一对应」类型）

| 步骤 | 位置 |
|------|------|
| 构建 context | `ai-handlers::SyncAIHandler::build_memory_context` → 返回 `Option<Context>` |
| 转为与 OpenAI 一一对应的消息列表 | `Context::to_messages(include_system, current_question)` → `Vec<prompt::ChatMessage>`；或 `prompt::format_for_model_as_messages(...)` 直接得到 `Vec<ChatMessage>` |
| 类型定义 | `prompt::MessageRole`（System / User / Assistant）、`prompt::ChatMessage { role, content }`，与 API 的 `role` / `content` 一一对应 |
| 组装并发送 | `telegram-bot-ai::TelegramBotAI::get_ai_response_with_messages(messages)` / `get_ai_response_stream_with_messages(messages, callback)`：内部 prepend 固定 system，再将 `ChatMessage` 转为 `ChatCompletionRequestMessage` |
| 实际 HTTP/JSON | `openai-client::OpenAIClient::chat_completion`（async-openai 序列化为 JSON） |

## 6. 可选扩展方案

若希望更清晰地区分「背景」与「当前轮」，可考虑：

- **方案 A**：保持现状，仅将 system 指令改为可配置（从配置或 Context 的 system_message 注入）。
- **方案 B**：多条 user 消息（部分模型支持）  
  - 第 1 条 user：仅 context 文本（标题 + 近期对话 + 语义参考 + 用户偏好）。  
  - 第 2 条 user：仅当前问题。  
  这样模型可显式区分「背景材料」与「本轮问题」。
- **方案 C**：在 system 消息中放入「行为说明」，在一条 user 消息中放入「当前上下文 + 用户提问」（与当前类似，但 system 内容可来自配置或 memory）。

当前实现等价于 **方案 A 的固定 system + 单条 user（context + 用户提问）**；若采用 B/C，只需在 `telegram-bot-ai` 中调整 `messages` 的组装方式，context 的**文本格式**仍可继续使用 `prompt::format_for_model` 的输出。

## 7. 小结

- **历史消息融入方式**：从存储 → `RecentMessagesStrategy` → `format_message(entry)` 得到 `"User: ..."` / `"Assistant: ..."` 文本，进入 `Context.recent_messages`，再经 `format_for_model` 放入「Conversation (recent):」段；**整段 context 作为一条 user 消息的 content 文本块**，并非多条 API 的 user/assistant 消息（见第 2 节）。
- **Context**：由 memory 构建，经 `prompt::format_for_model` 转为带 section 标题的纯文本。
- **发送方式**：通过 OpenAI Chat Completions 的 **JSON messages**，当前为 1 条 system + 1 条 user；**context 放在该 user 的 content 中**，与「用户提问: 」+ 当前问题拼接。
- **格式**：API 请求体为 JSON；context 在 content 内为**非 JSON 的约定文本格式**（见第 4 节），便于模型解析且与现有 prompt crate 一致。
