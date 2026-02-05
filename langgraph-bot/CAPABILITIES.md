# langgraph-bot Telegram Bot 能力说明

## 当前状态（2026-02-05）

### ✅ 已实现功能

#### 1. ReAct 对话
- **多轮持久化**：基于 thread_id（Telegram chat_id）的多轮对话
- **流式输出**：占位消息 → token 逐个更新 → 最终完整回复
- **思考-行动-观察循环**：ReAct 模式（Think → Act → Observe）
- **工具调用**：支持 MCP Exa 网络搜索工具

#### 2. 触发条件
- **回复机器人**：回复 bot 发送的任何消息，以整条消息内容作为问题
- **@提及机器人**：消息中包含 `@bot_username`，去掉 @ 后为问题
  - 若 @后无内容，使用默认提示："The user only @mentioned you with no specific question. Please greet them briefly and invite them to ask."

#### 3. 消息处理
- **同会话排队**：同一 chat_id 的消息串行处理，不拒绝新消息
- **独立会话**：不同 chat_id（私聊、群聊）互不干扰
- **流式编辑间隔**：可配置 `edit_interval_secs`（默认 1 秒）

#### 4. 用户身份
- **UserProfile 注入**：每轮对话通过 system prompt 注入用户信息
- **信息包含**：user_id、first_name、last_name、username
- **格式示例**：`User profile: John Doe (@johndoe), user_id: 123456789`

#### 5. 错误处理
- **用户友好提示**："处理时出错，请稍后再试。"
- **详细日志**：内部错误信息记录到日志
- **不泄露内部类型**：错误统一为 `anyhow::Error`

#### 6. 持久化
- **Checkpoint 存储**：SqliteSaver 存储对话历史
- **thread 隔离**：每个 Telegram chat 对应一个 thread
- **状态恢复**：新对话从 checkpoint 恢复历史上下文

### ✅ CLI 工具

| 命令 | 功能 |
|------|------|
| `info --db <path>` | 打印运行时配置（checkpointer 路径、模型、MCP 等） |
| `memory --db <path> [--thread-id <id>]` | 查看 checkpoint 状态（所有 thread 或特定 thread） |
| `seed --db <path> [--thread-id <id>]` | 生成 seed 消息到指定 thread |
| `load --db <path> --messages <json> [--thread-id <id>]` | 从 JSON 加载消息到 checkpoint |
| `chat --db <path> [message]` | 命令行对话模式（可选首条消息） |
| `run --db <path> [--token <token>]` | 启动 Telegram bot |

### ⏳ 待增强功能

#### 1. 高级 ReAct
- **自定义工具**：支持用户自定义工具（需扩展 ToolSource）
- **多工具并行**：ActNode 支持并发调用多个工具
- **工具链**：工具间依赖关系与链式调用

#### 2. 对话增强
- **对话总结**：长对话自动总结，减少 context 长度
- **对话分支**：支持 fork 对话线程
- **对话导出**：导出特定 thread 的对话历史

#### 3. 用户个性化
- **长期记忆**：结合长期记忆存储用户偏好
- **个性化 prompt**：基于用户画像动态调整 system prompt
- **记忆注入**：从长期记忆注入相关上下文

#### 4. 监控与运维
- **对话指标**：跟踪对话轮次、工具调用次数、响应时间
- **健康检查**：checkpoint 健康状态、LLM 连接状态
- **告警**：异常对话、错误率过高时告警

#### 5. 多模型支持
- **模型切换**：支持不同对话使用不同模型
- **模型回退**：主模型失败时自动回退到备用模型
- **成本控制**：基于 token 使用量的成本跟踪

## 使用示例

### 基本对话

```bash
# 1. 启动 bot
BOT_TOKEN=your_token OPENAI_API_KEY=your_key \
  cargo run -p langgraph-bot run --db checkpoint.db

# 2. 在 Telegram 中
# 方式 A：回复 bot 的消息
# 方式 B：@提及 bot
@your_bot 你好，请介绍一下自己

# 观察流式输出：
# "正在思考…" → "你好！我是你的AI助手..."
```

### 带网络搜索

```bash
# 设置 MCP Exa
EXA_API_KEY=your_exa_key MCP_EXA_URL=https://mcp.exa.ai/mcp \
  cargo run -p langgraph-bot run --db checkpoint.db

# 提问
@your_bot 今天北京天气怎么样？

# bot 会调用搜索工具，返回天气信息
```

### 查看历史

```bash
# 列出所有对话
cargo run -p langgraph-bot memory --db checkpoint.db

# 查看特定对话
cargo run -p langgraph-bot memory --db checkpoint.db --thread-id <chat_id>
```

### 预加载历史消息

```bash
# 1. 准备 messages.json
[
  {"id":"1","user_id":1,"chat_id":1,"username":"alice","first_name":"Alice","last_name":"","message_type":"text","content":"你好","direction":"received","created_at":"2025-02-01T10:00:00Z"},
  {"id":"2","user_id":2,"chat_id":1,"username":null,"first_name":null,"last_name":null,"message_type":"text","content":"你好！有什么我可以帮你的吗？","direction":"sent","created_at":"2025-02-01T10:00:15Z"}
]

# 2. 加载到 thread
cargo run -p langgraph-bot load --db checkpoint.db --thread-id test_thread --messages messages.json

# 3. bot 会基于此历史继续对话
```

## 技术架构

```
Telegram Message → AgentHandler → get_question()
                              ↓
                         run_chat_stream()
                              ↓
                         build_initial_state()
                              ↓
                      [checkpoint恢复 或 新建]
                              ↓
                    compiled.stream(ReAct图)
                              ↓
                    Think → Act → Observe (循环)
                              ↓
                         StreamEvent:
                            - Messages: on_chunk()
                            - Values: final_state
                              ↓
                    edit_message() 更新占位消息
```

## 关键文件

| 文件 | 作用 |
|------|------|
| `langgraph-bot/src/react.rs` | ReactRunner、create_react_runner、run_chat_stream |
| `langgraph-bot/src/telegram_handler.rs` | AgentHandler（Telegram 消息处理） |
| `langgraph-bot/src/run/mod.rs` | run_telegram（启动 bot） |
| `langgraph-bot/src/checkpoint.rs` | checkpoint 操作（导入、查询） |
| `langgraph-bot/tests/react_test.rs` | 单元测试 |

## 相关文档

- [更新方案与计划](../docs/langgraph-bot-update-plan.md) - 完整的实施计划
- [自测清单](self-test-checklist.md) - 验收标准与自测记录
- [记忆系统](../docs/memory/README.md) - 短期/长期记忆方案
