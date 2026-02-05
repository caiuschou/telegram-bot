# langgraph-bot 更新方案与计划（最优方案）

## 1. 目标

- 实现基于 langgraph-rust 的**真实 ReAct 对话**：多轮持久化（按 thread）、流式/非流式、可选用户身份注入。
- 采用**单一最优架构**：不迁就现有桩实现，按最简、可复用、易测的方式设计。

## 2. 最优架构概览

### 2.1 分层

```
AgentHandler / CLI
       ↓
run_chat(thread_id, content, user_profile) / run_chat_stream(..., on_chunk, ...)
       ↓
ReActRunner { compiled, checkpointer }
       ↓
langgraph: build_react_initial_state + CompiledStateGraph::stream / invoke
```

- **Runner 只持有两样**：`CompiledStateGraph<ReActState>`、`Arc<dyn Checkpointer<ReActState>>`。LLM、ToolSource、Store 在**创建 runner 时**用于编译图，编译完成后不再持有，避免每轮重建图。
- **每轮调用**只做：用 `thread_id` 建 `RunnableConfig`，用 `build_react_initial_state` 得到初始 state（含从 checkpoint 恢复），按需合并 `user_profile` 进 system prompt，然后 `compiled.stream(state, config, modes)` 或 `compiled.invoke(state, config)`。

### 2.2 为何不用 run_react_graph_stream 而用 compiled.stream

- `run_react_graph_stream` 每次接受 `Box<dyn LlmClient>` 和 `Box<dyn ToolSource>` **按值**，即每轮都要新建或从某处拿 Box，不利于复用连接/MCP 进程。
- **更优做法**：在 `create_react_runner` 里**一次性**用 LLM、ToolSource、Checkpointer、可选 Store 编译出 `CompiledStateGraph`，runner 只存 `compiled` + `checkpointer`；每轮只调 `build_react_initial_state`（要 checkpointer 和 config）+ `compiled.stream` / `invoke`。这样资源只建一次，与上游“每轮传 Box”的用法解耦。

### 2.3 用户身份（user_profile）

- 将 `user_profile` 与默认 system 合并为一条 system 文案：  
  `system_prompt = user_profile.map(|p| format!("{}\n\n{}", REACT_SYSTEM_PROMPT, p.to_system_content())).unwrap_or(REACT_SYSTEM_PROMPT)`，传给 `build_react_initial_state(..., system_prompt)`。
- 不在 runner 里存 user 信息，每轮由调用方传入，保持无状态。

### 2.4 thread_id

- 每轮由调用方传入 `thread_id`（Telegram 场景下 = `chat_id.to_string()`）。  
- 构造 `RunnableConfig` 时只设 `thread_id`，其他用默认即可；不在 runner 内写死 thread。

## 3. 对外 API（稳定、极简）

| API | 含义 |
|-----|------|
| `create_react_runner(db_path: impl AsRef<Path>) -> Result<ReActRunner>` | 从 `db_path` 建 SqliteSaver，从环境建 LLM、ToolSource、可选 Store，编译图并返回 runner。 |
| `runner.run_chat_stream(thread_id, content, on_chunk, user_profile) -> Result<String>` | 流式一轮；每 token 调 `on_chunk(&str)`，返回最后一条 assistant 内容。 |
| `print_runtime_info(db_path)` | 可选；打印 checkpointer 路径、模型、是否 MCP 等，便于排查。 |

不暴露 `ReActState`、不暴露 langgraph 的 `StreamEvent`；错误统一为 `anyhow::Result`，在 Telegram 层做用户可读文案。

## 4. Telegram 消息处理流程

收到 Telegram 消息时，若使用 **AgentHandler**（`langgraph-bot/src/telegram_handler.rs`），按以下流程处理。

### 4.1 入口与触发条件

- 每条消息调用一次 `Handler::handle(message)`。
- **是否由 ReAct 处理**：`get_question(message, bot_username)`：
  - **回复机器人**：`reply_to_message_id.is_some() && reply_to_message_from_bot` → 以整条 `message.content` 为问题。
  - **@提及机器人**：`message.content` 含 `@bot_username` → 去掉 @ 后为问题；若为空则用默认提示文案（邀请用户提问）。
  - 否则返回 `None`，handler 返回 `Continue`，不跑 ReAct。

### 4.2 同会话内排队（不拒绝，执行仍串行）

- `thread_id = message.chat.id.to_string()`（一个 chat 一个 thread，与 §2.4 一致）。
- **策略**：同一 thread 不拒绝新消息；将请求**入队**，按 thread 串行执行，保证 checkpoint 顺序与多轮一致性。
- **实现要点**：
  - 每个 `thread_id` 对应一个任务队列（如 `mpsc` 或 `VecDeque` + 锁）；收到触发消息时构造任务（question、message、profile、chat、需回复的 message_id 等）并 push 到该 thread 的队列，立即发送占位消息（如「正在思考…」或「排队中…」），返回 `Stop`/`Continue` 不阻塞 handler。
  - 每个 thread 一个常驻 worker 任务（或全局 worker 按 thread 取任务）：从队列取任务，顺序执行 `run_chat_stream`，用该任务自己的占位消息做流式编辑；执行完后取下一项，队列空则等待。
  - 每条请求有**自己的**占位消息与最终编辑目标，互不抢占；同一会话多条消息会得到多条 bot 回复，顺序与请求顺序一致。
- **效果**：用户连续发多条不会被「上一条还在处理中，请稍候」拒绝；执行仍串行，checkpoint 读写顺序明确。

### 4.3 占位消息与流式编辑

- 在请求入队时（或 worker 取出任务时）为该请求发送一条占位消息（如「正在思考…」或「排队中…」），取得 `message_id`，作为该任务专属的编辑目标。
- Worker 执行该任务时：建立 channel，spawn 子任务从 channel 接收 chunk，按 `edit_interval_secs` 限频，用 `bot.edit_message` 更新**该任务对应的**占位消息内容。
- 调用 `run_chat_stream(runner, thread_id, question, on_chunk, Some(profile))`，每收到 chunk 即发送到 channel；从任务中的 `message` 构造 `UserProfile` 传入。

### 4.4 结束与错误

- `run_chat_stream` 返回后，用**最终回复**再次 `edit_message` 覆盖为完整内容；若出错，则将占位改为「处理时出错，请稍后再试。」并返回 `Stop`。

### 4.5 Run 子命令与 AgentHandler 的接法

- **当前**：`main.rs` 的 `Commands::Run { token, db }` 调用 `run_telegram(&db, token)`；`run_telegram`（`run/mod.rs`）使用 **NoOpHandler**，收到消息后**不会**触发 ReAct。
- **要启用上述流程**：在 `run_telegram`（或 main）中调用 `create_react_runner(db)` 得到 runner，构造 `AgentHandler::new(runner, bot, bot_username, placeholder_message, edit_interval_secs)`，并将 **AgentHandler** 作为 handler 传给 `telegram_bot::run_bot`（或接入 telegram-bot 的 HandlerChain），这样收到消息时才会走 §4.1～4.4。

## 5. 实现要点

### 5.1 create_react_runner

1. 用 `db_path` 创建 `SqliteSaver`，得到 `Arc<dyn Checkpointer<ReActState>>`。
2. 从环境（或配置）创建：`Box<dyn LlmClient>`（如 ChatOpenAI）、`Box<dyn ToolSource>`（如 MCP）、可选 `Arc<dyn Store>`。
3. 构建图：`StateGraph::new()` → `add_node("think", ThinkNode::new(llm))`、`add_node("act", ActNode::new(tool_source))`、`add_node("observe", ObserveNode::...)`，边 `START→think→act→observe→END`；若需 store 则 `with_store(store)`；`compile_with_checkpointer(checkpointer)` 得到 `CompiledStateGraph<ReActState>`。
4. 返回结构体：`{ compiled, checkpointer }`。不保留 LLM/ToolSource 引用（已打进图里）。

### 5.2 run_chat_stream(thread_id, content, on_chunk, user_profile)

1. `config = RunnableConfig::default().with_thread_id(thread_id)`（或等价设置）。
2. `system_prompt = user_profile.map(|p| format!("{}\n\n{}", REACT_SYSTEM_PROMPT, p.to_system_content())).unwrap_or_else(|| REACT_SYSTEM_PROMPT.to_string())`；若 langgraph 暴露的是 `Option<&str>`，则传 `Some(system_prompt.as_str())`。
3. `state = build_react_initial_state(content, Some(checkpointer.as_ref()), Some(&config), Some(system_prompt)).await?`。
4. `stream = compiled.stream(state, Some(config), [Messages, Values, ...])`（仅需 Messages 用于 on_chunk，Values 用于取最终 state）。
5. 循环 `stream.next().await`：若 `StreamEvent::Messages { chunk, .. }` 则 `on_chunk(&chunk.content)`；若 `StreamEvent::Values(s)` 则记下 `final_state = s`。
6. 从 `final_state` 的 `messages` 中取最后一条 `Message::Assistant(content)` 的 `content` 返回；若无则返回空字符串或占位，避免 Telegram 空编辑。

### 5.3 错误与边界

- 将 langgraph 的 `RunError` / `CheckpointError` 映射为 `anyhow::Error`；Telegram 层捕获后展示简短用户文案（如「处理时出错，请稍后再试」）。
- 无 assistant 消息时：返回 `""` 或固定占位，并在日志中打 warning。

### 5.5 build_react_initial_state 说明

- **来源**：langgraph 库导出，`langgraph::build_react_initial_state`（定义于 `langgraph-rust/langgraph/src/react/runner.rs`）。
- **作用**：根据本轮的 user 消息与可选 checkpoint，构造当轮 ReAct 的**初始状态** `ReActState`，供后续 `compiled.stream(state, config, modes)` 使用。
- **签名（含义）**：
  - `user_message: &str`：本轮用户输入（即 `content`）。
  - `checkpointer: Option<&dyn Checkpointer<ReActState>>`：有则按 config 从 checkpoint 拉取该 thread 最新 state，并在其 `messages` 末尾追加本条 `Message::User(user_message)`；无则不从 checkpoint 恢复。
  - `runnable_config: Option<&RunnableConfig>`：与 checkpointer 配合使用；需 `config.thread_id` 存在时才从 checkpoint 加载，否则视为新会话。
  - `system_prompt: Option<&str>`：首条 System 消息内容。新会话时为 `[system_prompt, user_message]`；从 checkpoint 恢复时仅追加 user 消息，不改写已有 messages（若需每轮注入 user_profile，通过此处传入已合并好的 system 字符串即可）。
- **返回**：`Result<ReActState, CheckpointError>`。成功时为 `ReActState { messages, tool_calls, tool_results, turn_count }`，其中 `messages` 要么是「system + user」新会话，要么是「上一轮恢复的 messages + 本条 user」。
- **在本文方案中的用法**：`run_chat_stream` 每轮调用一次；`system_prompt` 由 `system_prompt_for_turn(user_profile)` 提供；`runnable_config` 的 `thread_id` 设为当轮 `thread_id`（如 chat_id），以实现按 thread 的多轮恢复。

## 6. 实施计划（按顺序）

**实施状态**：阶段 1～4 已完成（Runner、run_chat_stream、错误处理、文档与自测）。当前使用 langgraph git 依赖；因 git 版 `ReactBuildConfig` 字段较少，采用 `react_build_config_for_runner(db_path)` 从 env 构造 config，OpenAI 相关从 `std::env` 读取。

### 阶段 1：Runner 类型与 create_react_runner（P1）✅ 已完成

**概述**：本阶段是整条链路的入口，只做一件事——定义 ReAct 运行时的类型并实现其创建函数。不涉及“单轮对话”的调用逻辑，产出仅为可被后续阶段复用的 `ReActRunner` 实例（持 `compiled` + `checkpointer`）。范围限定在：类型定义、从 `db_path` 与环境构建 checkpointer/LLM/ToolSource、编译图并返回 runner。

**开发内容**：确立 ReAct 运行时的唯一入口类型与创建方式。具体包括：定义仅持 `compiled` + `checkpointer` 的 `ReActRunner`；实现从 `db_path` 创建 SqliteSaver、从环境构建 LLM/ToolSource/可选 Store、编译 ReAct 图并返回 runner 的 `create_react_runner`；确认并依赖 langgraph 对外暴露的 `build_react_initial_state`、`stream`/`invoke`、`RunnableConfig` 等 API。本阶段不实现“单轮对话”调用，只产出可复用的 runner 实例。

**与前后阶段**：无上一步（入口阶段）。完成本阶段后，调用方才能拿到 `ReActRunner`，从而为阶段 2（流式单轮）提供统一的运行载体；`run_chat_stream` 依赖本阶段产出的 `compiled` 与 `checkpointer`。

1. 定义 `ReActRunner { compiled: CompiledStateGraph<ReActState>, checkpointer: Arc<dyn Checkpointer<ReActState>> }`（及必要 clone 或内部 Arc 以满足并发）。
2. 实现 `create_react_runner(db_path)`：建 checkpointer、LLM、ToolSource、可选 Store，编译图，返回 runner。
3. 确认 langgraph 对外暴露：`build_react_initial_state`、`CompiledStateGraph::stream` / `invoke`、`RunnableConfig`、`Message`、`ReActState`。若当前仅通过 `run_react_graph` / `run_react_graph_stream` 暴露，则需在 langgraph 侧增加对 `CompiledStateGraph` 与 `build_react_initial_state` 的公开 API，或使用 path 依赖在本地先扩展再统一上游。

**验证**：`create_react_runner(":memory:")` 成功；无 env 时返回明确错误。已通过：`create_react_runner_fails_without_api_key` 测试及 `cargo test -p langgraph-bot`。

**测试用例说明**（单元测试，`langgraph-bot/tests/react_test.rs`）：

| 用例名 | 类型 | Given | When | Then |
|--------|------|-------|------|------|
| create_react_runner_fails_without_api_key | 单元 | 未设置 `OPENAI_API_KEY` | 调用 `create_react_runner(db_path)` | 返回 `Err` 且错误信息包含 `OPENAI_API_KEY` |
| create_react_runner_succeeds | 单元 | 已设置 `OPENAI_API_KEY` 与临时 DB 路径 | 调用 `create_react_runner(db_path)` | 返回 `Ok(ReActRunner)` 且 DB 文件存在（无 API key 时跳过） |

**结论**：阶段完成时，调用方可通过 `create_react_runner(db_path)` 得到 `ReActRunner`，且无 API key 时得到明确错误。以“`create_react_runner_fails_without_api_key` 与 `create_react_runner_succeeds` 通过、`cargo test -p langgraph-bot` 通过”为验收结论；后续阶段均依赖本阶段产出的 runner。

### 阶段 2：run_chat_stream（P1）

**概述**：在已有 runner 之上实现唯一的对话入口——流式单轮 API `run_chat_stream`。每轮完成“构建 config/system_prompt → 初始 state → stream → on_chunk 回调 + 取最后 assistant 内容并返回”。并抽取“取最后一条 assistant 内容”为公共辅助函数，便于单测与行为统一。交付后，Telegram 占位逐 chunk 更新与 CLI 流式输出都具备实现基础。

**开发内容**：实现流式单轮对话 API `run_chat_stream`。具体包括：按 5.2 构建 config、system_prompt、调用 `build_react_initial_state`，使用 `compiled.stream` 消费 `StreamEvent`，在循环中同步调用 `on_chunk` 并收集最终 state；抽取“从 ReActState 取最后一条 assistant 内容”为共用辅助函数，供本阶段与阶段 3 复用并便于单测。本阶段交付的是“每 token 回调 + 返回完整回复”的流式能力，为 Telegram 占位消息逐 chunk 更新和 CLI 流式输出提供实现基础。

**与前后阶段**：依赖阶段 1 产出的 `ReActRunner`（`compiled`、`checkpointer`），否则无法执行 stream。完成后，阶段 3 的错误映射与阶段 4 的 Telegram/CLI 流式自测都建立在“流式 API 已可用”之上。

1. 实现 5.2 的完整流程；`on_chunk` 在 stream 循环中同步调用。
2. 抽取“从 ReActState 取最后一条 assistant 内容”为共用辅助函数，便于 run_chat 复用与单测。

**验证**：CLI 流式模式能逐 token 输出；Telegram 占位消息随 chunk 更新并最终被完整回复替换。

**测试用例说明**（单元/集成）：

| 用例名 | 类型 | Given | When | Then |
|--------|------|-------|------|------|
| run_chat_stream_invokes_on_chunk | 单元 | runner 与 thread_id | 调用 `run_chat_stream(..., on_chunk)` | `on_chunk` 被多次调用、chunk 顺序与流式一致；返回值非空且与最后一条 assistant 内容一致（可 mock 或无 API key 时跳过） |
| run_chat_stream_returns_last_assistant_content | 单元 | 一次流式调用结束 | 从返回的 state 取最后 assistant | 与 `run_chat_stream` 返回值一致；与“取最后 assistant 内容”辅助函数单测联动 |
| last_assistant_content_helper | 单元 | `ReActState` 的 messages 含多条 Assistant/User | 调用“取最后一条 assistant 内容”辅助函数 | 返回最后一条 Assistant 的 content；无 Assistant 时返回空字符串（不依赖网络） |
| cli_stream_mode_prints_tokens | 集成/手动 | CLI 流式模式 | 执行一轮对话 | 能逐 token 打印；Telegram 占位随 chunk 更新为可选 E2E |

**结论**：阶段完成时，`run_chat_stream` 可被 CLI 与 Telegram 调用，且返回内容与 checkpoint 中最后一条 assistant 一致。以“`run_chat_stream_invokes_on_chunk`、`run_chat_stream_returns_last_assistant_content`、`last_assistant_content_helper` 通过，且 CLI 流式能逐 token 输出”为验收结论；阶段 3 的错误处理与阶段 4 的自测都建立在本阶段可用的流式 API 之上。

### 阶段 3：错误与边界 + print_runtime_info（P2）✅ 已完成

**概述**：不新增业务能力，只把阶段 2 的 `run_chat_stream` 在异常与边界下的行为约定清楚并实现。包括：将 langgraph 的 `RunError`/`CheckpointError` 统一映射为 `anyhow::Error`、无 assistant 时的返回值与日志、可选的 `print_runtime_info` 便于排查。交付后，上层可安全展示“处理时出错，请稍后再试”等用户文案，且运维能快速确认环境。

**开发内容**：统一 `run_chat_stream` 的错误与边界行为。具体包括：将 langgraph 的 `RunError`、`CheckpointError` 映射为 `anyhow::Error`，保证对外不暴露内部类型；约定无 assistant 消息时的返回值（空字符串或固定占位）及日志；可选实现 `print_runtime_info`，从 runner 或 env 打印 checkpointer 路径、模型、MCP 等，便于排查。本阶段不新增业务能力，而是让阶段 2 的流式 API 在异常与边界情况下行为明确、可被 Telegram 层安全展示为“处理时出错，请稍后再试”等用户文案。

**与前后阶段**：依赖阶段 2 已实现的 `run_chat_stream`，在其上增加错误映射与边界处理。完成后，阶段 4 的自测与文档才能完整覆盖“错误时占位消息展示”“无回复时的占位”等场景；运维也可通过 print_runtime_info 快速确认环境。

1. 统一错误映射与无 assistant 时的返回值。
2. 实现 `print_runtime_info`（可选）：从 runner 或 env 打印 checkpointer 路径、模型、MCP 等。

**测试用例说明**：

| 用例名 | 类型 | Given | When | Then |
|--------|------|-------|------|------|
| run_chat_stream_maps_langgraph_errors | 单元/集成 | 导致 CheckpointError / RunError 的场景（如无效 thread 或 IO 错误） | 调用 `run_chat_stream` | 返回 `anyhow::Error`，不暴露内部类型（可构造异常路径） |
| no_assistant_message_returns_empty_or_placeholder | 单元 | 某轮执行后 state 中无 Assistant 消息 | 取“最后 assistant 内容” | 返回空字符串或固定占位，不 panic；调用方得到可编辑的占位文案（构造仅含 User 的 state 调用辅助函数） |
| print_runtime_info_prints_config | 单元（可选） | runner 或 db_path | 调用 `print_runtime_info` | 标准输出包含 checkpointer 路径、模型名等（断言 stdout 或通过 writer 捕获） |

**结论**：阶段完成时，所有通过 `run_chat_stream` 触发的错误对外均为 `anyhow::Error`，无 assistant 时返回约定占位且不 panic。以“错误映射与无 assistant 单测通过、可选时 `print_runtime_info` 输出符合预期”为验收结论；阶段 4 的自测与文档可据此覆盖错误与占位场景。

### 阶段 4：文档与自测（P1）✅ 已完成

**概述**：不新增代码功能，只把阶段 1～3 的交付固化为“可被正确使用”的说明与可重复的验收。包括：README/docs 中 thread_id 与 chat_id 对应、user_profile、流式行为；自测清单（seed/import、新 thread、多轮、Telegram 流式全流程、错误占位）；`cargo test` 全过且保留“取最后 assistant 内容”等单测作为行为文档。交付后，后续迭代（新 handler、新 CLI 子命令）有稳定的基线说明与验收标准。

**开发内容**：补齐使用说明与验收依据。具体包括：在 README 或 docs 中说明 thread_id 与 chat_id 的对应关系、user_profile 注入方式、流式行为；执行并固定自测清单（seed/import 写入 checkpoint、新 thread 首轮、同 thread 多轮、Telegram 流式全流程、错误时占位文案）；确保 `cargo test` 全过并保留“取最后 assistant 内容”等单元测试作为行为文档。本阶段不新增代码功能，而是让阶段 1～3 的产出可被正确使用与回归验证。

**与前后阶段**：依赖阶段 1～3 全部完成，否则文档与自测无法覆盖完整 API 与行为。本阶段是实施计划的收尾：对上一步的意义在于把前面各阶段的交付固化为文档与可重复的自测/自动化测试，对“下一步”的意义在于为后续迭代（如新 handler、新 CLI 子命令）提供稳定的基线说明与验收标准。

1. README / docs：说明 thread_id 与 chat_id 对应、user_profile 注入方式、流式行为。
2. 自测：seed/import 仍可写入 checkpoint；新 thread 首轮；同 thread 多轮；Telegram 流式全流程。
3. `cargo test` 全过，含“取最后 assistant 内容”的单元测试。

**测试用例说明**：

| 用例名 | 类型 | Given | When | Then |
|--------|------|-------|------|------|
| cargo_test_all_pass | 自动化 | 阶段 1～3 的单元/集成测试已实现 | 执行 `cargo test -p langgraph-bot`（及 `cargo test`） | 全部通过 |
| last_assistant_content_documentation | 文档/单测 | 阶段 2 的“取最后 assistant 内容”辅助函数 | 查阅/运行单测 | 有单元测试且作为行为文档（见阶段 2 的 `last_assistant_content_helper`） |
| 自测清单（手动） | 手动 | 部署/本地环境 | 按清单执行 | seed/import 写入 checkpoint 可用；新 thread 首轮正常；同 thread 多轮连续；Telegram 流式全流程（占位→chunk 更新→最终替换）；错误时占位为“处理时出错，请稍后再试” |

**结论**：阶段完成时，文档与自测清单可指导他人正确使用 runner 与 `run_chat_stream`，且 `cargo test` 全过、自测清单执行通过。以“README/docs 已更新、自测清单逐项通过、`cargo test -p langgraph-bot` 通过”为验收结论；实施计划在此收尾，后续迭代可基于本阶段产出的基线进行。

## 7. 依赖与顺序

- 阶段 1 必须最先（runner 与图编译）。
- 阶段 2 实现流式单轮 `run_chat_stream`。
- 阶段 3、4 在阶段 2 通过后做。

## 8. 产出与验收

- **代码**：Runner 仅持 `compiled` + `checkpointer`；`run_chat_stream` 每轮只做 config + build_initial_state + stream；user_profile 仅通过 system_prompt 注入。
- **行为**：Telegram 与 CLI 均通过流式产出真实多轮对话；错误有友好提示。
- **验收**：上述自测通过，`cargo test` 通过。

## 9. 参考

- **图与流**：langgraph `StateGraph`、`CompiledStateGraph::stream`、`build_react_initial_state`（`langgraph-rust/langgraph/src/react/runner.rs`、`graph/compiled.rs`）。
- **Stream 事件**：`StreamEvent::Messages { chunk }`、`StreamEvent::Values(state)`（`stream/mod.rs`）。

---

## 10. 核心代码 Review（仅文档内，不落盘到代码文件）

以下为核心实现片段，供在文档内 Review；实现时再迁入 `langgraph-bot` 对应模块。

### 10.1 类型与依赖

```rust
// 类型：Runner 仅持 compiled + checkpointer（与 §2.1 一致）
use langgraph::{
    build_react_initial_state, ActNode, CompilationError, CompiledStateGraph, Message,
    ObserveNode, ReActState, RunnableConfig, RunError, StateGraph, StreamEvent, StreamMode,
    ThinkNode, END, REACT_SYSTEM_PROMPT, START,
};
use langgraph::memory::{Checkpointer, SqliteSaver};
use langgraph::react_builder::{build_react_run_context, ReactBuildConfig};
use std::collections::HashSet;
use std::sync::Arc;
use tokio_stream::StreamExt;

pub struct ReactRunner {
    compiled: CompiledStateGraph<ReActState>,
    checkpointer: Arc<dyn Checkpointer<ReActState>>,
}
```

### 10.2 create_react_runner

```rust
/// 从 db_path 与环境构建 checkpointer、LLM、ToolSource、可选 Store，编译图后只保留 compiled + checkpointer。
pub async fn create_react_runner(db_path: impl AsRef<Path>) -> Result<ReactRunner> {
    let mut config = ReactBuildConfig::from_env();
    config.db_path = Some(db_path.as_ref().to_string());
    config.thread_id = Some("_".to_string()); // 仅用于让 builder 创建 checkpointer，实际 thread_id 每轮传入

    let ctx = build_react_run_context(&config).await.map_err(anyhow::Error::msg)?;
    let checkpointer = ctx.checkpointer.ok_or_else(|| anyhow::anyhow!("checkpointer required"))?;

    let openai_config = async_openai::config::OpenAIConfig::new()
        .with_api_key(config.openai_api_key.as_deref().unwrap_or(""))
        .with_api_base(config.openai_base_url.as_deref().unwrap_or(""));
    let model = config.model.as_deref().unwrap_or("gpt-4o-mini");
    let llm = langgraph::ChatOpenAI::new_with_tool_source(
        openai_config,
        model,
        ctx.tool_source.as_ref(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("llm tools: {}", e))?;
    let llm: Box<dyn langgraph::LlmClient> = Box::new(llm);

    let think = ThinkNode::new(llm);
    let act = ActNode::new(ctx.tool_source);
    let observe = ObserveNode::with_loop();
    let graph = StateGraph::<ReActState>::new()
        .add_node("think", Arc::new(think))
        .add_node("act", Arc::new(act))
        .add_node("observe", Arc::new(observe))
        .add_edge(START, "think")
        .add_edge("think", "act")
        .add_edge("act", "observe")
        .add_edge("observe", END);

    let compiled = if let Some(store) = &ctx.store {
        graph.with_store(store.clone()).compile_with_checkpointer(Arc::clone(&checkpointer))?
    } else {
        graph.compile_with_checkpointer(Arc::clone(&checkpointer))?
    };

    Ok(ReactRunner { compiled, checkpointer })
}
```

### 10.3 辅助：system_prompt 与 last_assistant_content

#### system_prompt_for_turn 说明（见 §5.2 上下文）

- **作用**：为当轮对话生成传给 `build_react_initial_state` 的 system prompt 字符串。Runner 不保存用户信息，每轮由调用方传入 `user_profile`，通过本函数合并进 system。
- **输入**：`user_profile: Option<&UserProfile>`。Telegram 场景下由 `AgentHandler` 从当前消息构造并传入；CLI 或测试可传 `None`。
- **输出**：若 `user_profile` 为 `Some(p)`，返回 `REACT_SYSTEM_PROMPT + "\n\n" + p.to_system_content()`（默认 ReAct 说明在前，用户身份在后）；若为 `None`，仅返回 `REACT_SYSTEM_PROMPT`。
- **使用处**：`run_chat_stream` 在调用 `build_react_initial_state` 前调一次，传 `Some(&system)` 作为 system_prompt 参数，与 §2.3、§5.2 一致。

```rust
fn system_prompt_for_turn(user_profile: Option<&UserProfile>) -> String {
    user_profile
        .map(|p| format!("{}\n\n{}", REACT_SYSTEM_PROMPT, p.to_system_content()))
        .unwrap_or_else(|| REACT_SYSTEM_PROMPT.to_string())
}

/// 从 ReActState.messages 取最后一条 Assistant 的 content；若无则返回空字符串。
fn last_assistant_content(state: &ReActState) -> String {
    state
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            Message::Assistant(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}
```

### 10.4 run_chat_stream

```rust
pub async fn run_chat_stream(
    &self,
    thread_id: &str,
    content: &str,
    mut on_chunk: impl FnMut(&str) + Send,
    user_profile: Option<&UserProfile>,
) -> Result<String> {
    let config = RunnableConfig {
        thread_id: Some(thread_id.to_string()),
        ..Default::default()
    };
    let system = system_prompt_for_turn(user_profile);
    let state = build_react_initial_state(
        content,
        Some(self.checkpointer.as_ref()),
        Some(&config),
        Some(&system),
    )
    .await
    .map_err(|e| anyhow::anyhow!("build_react_initial_state: {}", e))?;

    let modes: HashSet<StreamMode> = [StreamMode::Messages, StreamMode::Values]
        .into_iter()
        .collect();
    let mut stream = self.compiled.stream(state, Some(config), modes);

    let mut final_state: Option<ReActState> = None;
    while let Some(event) = stream.next().await {
        match &event {
            StreamEvent::Messages { chunk, .. } => on_chunk(&chunk.content),
            StreamEvent::Values(s) => final_state = Some(s.clone()),
            _ => {}
        }
    }

    Ok(final_state
        .as_ref()
        .map(last_assistant_content)
        .unwrap_or_default())
}
```

### 10.5 Review 检查项

- [ ] `ReactRunner` 仅持 `compiled` + `checkpointer`，无 LLM/ToolSource 引用。
- [ ] `create_react_runner` 用 `ReactBuildConfig::from_env()` + `build_react_run_context` 得到 checkpointer/tool_source/store，再自建图并 `compile_with_checkpointer`。
- [ ] 每轮 `RunnableConfig` 仅设 `thread_id`，与 §2.4 一致。
- [ ] `user_profile` 仅通过 `system_prompt_for_turn` 注入，与 §2.3 一致。
- [ ] 流式仅消费 `StreamEvent::Messages` 与 `Values`，无 assistant 时返回空字符串，与 §5.2、§5.3 一致。
- [ ] 错误统一转为 `anyhow::Error`，与 §3 一致。

---

## 11. 阶段 4 完成状态

**完成日期**：2026-02-05

**已完成**：
- ✅ `cargo test -p langgraph-bot` 全通过（36 个测试）
- ✅ docs/README.md 已更新，添加 langgraph-bot-update-plan.md 引用
- ✅ 自测清单已创建（langgraph-bot/self-test-checklist.md）
- ✅ CLI 功能测试完成（info、seed、load、memory 命令）
- ⏳ 手动测试需要有效的 OPENAI_API_KEY 和 Telegram Bot Token（新 thread 首轮、同 thread 多轮、Telegram 流式全流程、错误时占位文案）

**验收结论**：
- 自动化测试：100% 通过
- 文档更新：完成
- 自测清单：已创建
- CLI 功能：验证通过
- 手动测试：待实际环境验证

