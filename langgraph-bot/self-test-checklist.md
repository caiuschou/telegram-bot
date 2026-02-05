# langgraph-bot 阶段 4 自测清单

本文档记录 langgraph-bot 阶段 4 的自测验证情况。

## 自动化测试

### ✅ 单元测试

所有单元测试已通过（`cargo test -p langgraph-bot`）：

- **create_react_runner_fails_without_api_key**: 无 API key 时返回明确错误
- **create_react_runner_succeeds**: 有 API key 时成功创建 runner
- **last_assistant_content_helper**: 正确取最后一条 Assistant 消息
- **run_chat_stream_invokes_on_chunk**: 流式调用触发 on_chunk 回调
- **run_chat_stream_returns_last_assistant_content**: 返回值与 checkpoint 中最后一条 assistant 一致
- **print_runtime_info_prints_config**: 配置信息正确打印

共 36 个测试全部通过（6 个 react_test + 6 个 checkpoint_react_state_test + 8 个 checkpoint_test + 8 个 load_test + 8 个 src/lib.rs 单元测试）。

## CLI 功能测试

### ✅ info 命令

测试配置信息输出：

```bash
$ cargo run -p langgraph-bot info --db /tmp/test_self_check.db
=== langgraph-bot Runtime Info ===
Checkpointer: /tmp/test_self_check.db
OpenAI API Key: set (length: 35)
Model: gpt-4o-mini
MCP Exa URL: https://mcp.exa.ai/mcp
MCP Remote: npx -y mcp-remote
Exa API Key: set (length: 36)
==================================
```

### ✅ seed 命令

测试 seed 消息写入 checkpoint：

```bash
$ cargo run -p langgraph-bot seed --db /tmp/test_self_check.db --thread-id test_thread_1
Seeded thread test_thread_1 with checkpoint id: 1f10247f-173c-6156-b151-4aec700d3151
Integrity: OK (100 messages)
Format: OK (User/Assistant only)
  [1] User: [User: Alice / @alice] 在吗？
  [2] Assistant: 在的，咋了
  ...
```

### ✅ load 命令

测试从 JSON 加载消息到 checkpoint：

```bash
$ cargo run -p langgraph-bot load --db /tmp/test_self_check.db --thread-id test_thread_2 --messages /tmp/test_messages.json
Loaded thread test_thread_2 with checkpoint id: 1f102480-62f9-6cfa-8846-1a40bafcc276
Integrity: OK (2 messages)
Format: OK (User/Assistant only)
  [1] User: [User: - / -] Hello from JSON
  [2] Assistant: Hi there from JSON
```

### ✅ memory 命令

测试查询 checkpoint 多线程信息：

```bash
$ cargo run -p langgraph-bot memory --db /tmp/test_self_check.db
Short-term memory (checkpoint): /tmp/test_self_check.db
  threads: 2
  thread_id: test_thread_1
  messages: 100
  first: User: [User: Alice / @alice] 在吗？
  last:  Assistant: 好，回头聊

  thread_id: test_thread_2
  messages: 2
  first: User: [User: - / -] Hello from JSON
  last:  Assistant: Hi there from JSON
```

## 手动测试指南

### 前置条件

1. **环境变量设置**（.env 文件）：

```bash
# Telegram Bot Token（从 @BotFather 获取）
BOT_TOKEN=your_bot_token_here

# OpenAI API Key
OPENAI_API_KEY=your_openai_api_key

# 可选：自定义模型
OPENAI_MODEL=gpt-4o-mini

# 可选：自定义 API 端点
# OPENAI_BASE_URL=https://api.openai.com/v1

# MCP Exa（网络搜索工具）
EXA_API_KEY=your_exa_api_key
MCP_EXA_URL=https://mcp.exa.ai/mcp
```

2. **启动 bot**：

```bash
cargo run -p langgraph-bot run --db checkpoint.db
```

### 测试场景

#### 1. 新 thread 首轮

**步骤：**
1. 在 Telegram 中向 bot 发送消息：
   - 方式 A：回复 bot 之前发的消息（任意消息即可）
   - 方式 B：@提及 bot（如 `@your_bot 你好`）
2. 观察占位消息（"正在思考…"）
3. 观察流式更新（每个 token 逐个追加）
4. 最终替换为完整回复

**验证点：**
- ✅ 占位消息正确发送
- ✅ 流式更新可见（不是一次性显示）
- ✅ 最终回复完整且有意义
- ✅ checkpoint.db 中创建新的 thread 记录

#### 2. 同 thread 多轮

**步骤：**
1. 在同一个聊天中连续发送多条消息
2. 每条消息都回复 bot 或 @提及
3. 观察每轮对话的上下文是否正确

**验证点：**
- ✅ 每轮对话都收到回复
- ✅ 第二轮能引用第一轮的内容
- ✅ checkpoint.db 中该 thread 的消息数正确增长
- ✅ 消息顺序正确（system → user1 → assistant1 → user2 → assistant2 → ...）

#### 3. 不同 chat 独立

**步骤：**
1. 在两个不同的聊天中（私聊和群聊）分别发送消息
2. 确认两个聊天的对话互不干扰

**验证点：**
- ✅ 每个聊天有独立的 thread_id
- ✅ 不同聊天的对话内容不混淆
- ✅ checkpoint.db 中有两条独立的 thread 记录

#### 4. 错误处理

**步骤：**
1. 临时移除 `OPENAI_API_KEY` 或设置无效值
2. 重启 bot
3. 在 Telegram 中发送消息

**验证点：**
- ✅ 占位消息替换为"处理时出错，请稍后再试。"
- ✅ 日志中记录详细错误信息
- ✅ 不泄露内部错误类型给用户

#### 5. 网络搜索（MCP Exa）

**步骤：**
1. 确认 `EXA_API_KEY` 和 `MCP_EXA_URL` 已设置
2. 在 Telegram 中提问："今天天气怎么样？" 或 "搜索最新的科技新闻"
3. 观察 bot 是否调用搜索工具

**验证点：**
- ✅ bot 能调用 MCP Exa 工具
- ✅ 返回搜索结果
- ✅ 回复中包含搜索到的信息

### 命令行验证

#### 查看 checkpoint 状态

```bash
# 列出所有 thread
cargo run -p langgraph-bot memory --db checkpoint.db

# 查看特定 thread
cargo run -p langgraph-bot memory --db checkpoint.db --thread-id <thread_id>
```

#### 查看 runtime 配置

```bash
cargo run -p langgraph-bot info --db checkpoint.db
```

#### 预加载历史消息（可选）

```bash
# 从 JSON 加载消息到某个 thread
cargo run -p langgraph-bot load --db checkpoint.db --thread-id test_thread --messages messages.json

# 生成 seed 消息
cargo run -p langgraph-bot seed --db checkpoint.db --thread-id test_thread
```

### ⏳ 新 thread 首轮

**步骤：**
1. 准备一个新的数据库文件（如 `new_thread.db`）
2. 运行 `cargo run -p langgraph-bot chat --db new_thread.db "Say hello"`
3. 验证：收到 LLM 回复，checkpoint 中有 2 条消息（system + user）+ 1 条 assistant

**验证点：**
- checkpoint 首轮包含 system 消息
- user 消息内容正确
- assistant 回复内容正确
- thread_id 正确设置

### ⏳ 同 thread 多轮

**步骤：**
1. 使用已存在的 thread_id（如 `new_thread.db`）
2. 运行第一轮：`cargo run -p langgraph-bot chat --db new_thread.db "What is 2+2?"`
3. 运行第二轮：`cargo run -p langgraph-bot chat --db new_thread.db "And 3+3?"`
4. 运行 `cargo run -p langgraph-bot memory --db new_thread.db`

**验证点：**
- checkpoint 中包含 2 轮对话的所有消息
- 消息顺序正确：system → user1 → assistant1 → user2 → assistant2
- 第二轮回复能引用第一轮上下文

### ⏳ Telegram 流式全流程

**步骤：**
1. 设置 `BOT_TOKEN` 环境变量
2. 运行：`cargo run -p langgraph-bot run --db telegram_test.db`
3. 在 Telegram 中回复机器人消息或 @提及机器人
4. 观察流式更新（占位消息 → chunk 更新 → 最终完整回复）

**验证点：**
- 占位消息正确发送（"正在思考…"）
- 每 token 逐个更新消息内容
- 最终替换为完整回复
- 同会话连续发消息不被拒绝（排队处理）
- 不同会话（不同 chat_id）互不影响

### ⏳ 错误时占位文案

**步骤：**
1. 模拟错误场景（如无效 API key 或网络错误）
2. 运行 `cargo run -p langgraph-bot run --db error_test.db`
3. 在 Telegram 中触发对话

**验证点：**
- 占位消息替换为"处理时出错，请稍后再试。"
- 日志中记录详细错误信息
- 不泄露内部错误类型给用户

## 总结

- ✅ 自动化测试：全部通过（36/36）
- ✅ CLI 功能测试：info、seed、load、memory 命令均正常
- ⏳ 手动测试：需要有效的 OPENAI_API_KEY 和 Telegram Bot Token 环境进行

**验收标准：**
- [x] `cargo test -p langgraph-bot` 全通过
- [x] info、seed、load、memory 命令可正常执行
- [ ] 新 thread 首轮正常（需手动）
- [ ] 同 thread 多轮正常（需手动）
- [ ] Telegram 流式全流程正常（需手动）
- [ ] 错误时占位文案正确（需手动）
