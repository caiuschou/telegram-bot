# Context 发送方案（OpenAI 对话）

约定：**记忆上下文如何组装并以 JSON 消息形式发送到 OpenAI Chat Completions**。

## 目标

- context 组成与文本格式（如 "Conversation (recent):\nUser: ...\nAssistant: ..."）。
- 与 API 一一对应：`Context::to_messages` 得到 `Vec<ChatMessage>`，对应请求体 `messages` 数组的 `{ "role", "content" }`。

## 历史消息来源与融入

| 步骤 | 位置 | 说明 |
|------|------|------|
| 1 | MemoryStore | 存 user/assistant 消息，带 role、content |
| 2 | RecentMessagesStrategy | search_by_conversation/user，取最近 N 条 |
| 3 | format_message | 单条格式化为 "User:" / "Assistant:" + content |
| 4 | ContextBuilder::build | 策略结果进 Context.recent_messages |
| 5 | format_for_model / to_messages | 拼成 "Conversation (recent):\n..." 再转 Vec<ChatMessage> |
| 6 | SyncAIHandler → get_ai_response_with_messages | 作为 messages 数组发给 OpenAI |

融入方式：**多条 API 消息**（system / user / assistant 轮次）。当前实现见 `SyncAIHandler::build_messages_for_ai`、`Context::to_messages`、prompt 模块。
