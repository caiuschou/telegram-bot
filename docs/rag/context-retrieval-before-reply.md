# AI 回答前上下文检索

## 概述

AI 在生成回复之前，会先根据用户当前问题查询记忆库中的相关上下文（最近消息、语义相关历史、用户偏好），再将这些上下文与问题一起交给模型生成回复。本文档说明该流程涉及的代码位置与调用关系。

## 索引

- [AI 回答前上下文检索](#ai-回答前上下文检索)
  - [概述](#概述)
  - [索引](#索引)
  - [流程概览](#流程概览)
  - [入口：AI 回答前先构建上下文](#入口ai-回答前先构建上下文)
    - [普通模式（handle\_query\_normal）](#普通模式handle_query_normal)
    - [流式模式（handle\_query\_streaming）](#流式模式handle_query_streaming)
  - [上下文构建：build\_memory\_context](#上下文构建build_memory_context)
    - [策略配置](#策略配置)
    - [代码摘要](#代码摘要)
  - [语义检索：SemanticSearchStrategy](#语义检索semanticsearchstrategy)
    - [流程](#流程)
    - [与外部组件的交互](#与外部组件的交互)
    - [关键代码](#关键代码)
  - [策略执行：ContextBuilder.build](#策略执行contextbuilderbuild)
  - [代码位置汇总](#代码位置汇总)
  - [测试用例建议](#测试用例建议)
    - [断言补充建议](#断言补充建议)
  - [相关文档](#相关文档)

---

## 流程概览

```
用户 @ 提及机器人并提问
    ↓
SyncAIHandler.handle() 识别为 AI 查询
    ↓
process_normal / process_streaming
    ↓
build_memory_context(user_id, conversation_id, question)   ← 先查相关上下文
    ↓
ContextBuilder 依次执行策略：
  - RecentMessagesStrategy：最近 N 条消息
  - SemanticSearchStrategy：用 question 做语义检索（embed + semantic_search）
  - UserPreferencesStrategy：用户偏好
    ↓
format_question_with_context(question, context)
    ↓
get_ai_response(question_with_context) / get_ai_response_stream(...)   ← 再调用 AI
    ↓
发送回复并写入记忆
```

**结论**：无论是普通模式还是流式模式，都是**先查询/构建上下文，再调用 AI 接口**。

---

## 入口：AI 回答前先构建上下文

**文件**：`ai-handlers/src/sync_ai_handler.rs`

### 普通模式（process_normal）

在处理每条 AI 查询时，先调用 `build_memory_context` 得到上下文，再调用 `get_ai_response`。

| 行号（约） | 说明 |
|-----------|------|
| 159-185   | `process_normal`：先 `build_memory_context`，再 `format_question_with_context`，最后 `get_ai_response` |

相关代码逻辑：

```rust
let context = self
    .build_memory_context(&user_id_str, &conversation_id_str, &query.question)
    .await;
let question_with_context = self.format_question_with_context(&query.question, &context);

match self.ai_bot.get_ai_response(&question_with_context).await {
    // ...
}
```

### 流式模式（process_streaming）

同样先构建上下文，再流式请求 AI。

| 行号（约） | 说明 |
|-----------|------|
| 189-256   | `process_streaming`：先 `build_memory_context`，再 `format_question_with_context`，最后 `get_ai_response_stream` |

相关代码逻辑：

```rust
let context = self
    .build_memory_context(&user_id_str, &conversation_id_str, &query.question)
    .await;
let question_with_context = self.format_question_with_context(&query.question, &context);
// ...
match self.ai_bot.get_ai_response_stream(&question_with_context, |chunk| { ... })
```

---

## 上下文构建：build_memory_context

**文件**：`ai-handlers/src/sync_ai_handler.rs`（约 82-114 行）

`build_memory_context` 负责「查相关上下文」的编排：使用 `ContextBuilder` 配置多种策略，其中包含**语义检索**。

### 策略配置

| 策略 | 作用 |
|------|------|
| `RecentMessagesStrategy::new(10)` | 最近 10 条消息 |
| **`SemanticSearchStrategy::new(5, embedding_service)`** | **用当前问题做语义检索，取 5 条最相关记忆** |
| `UserPreferencesStrategy::new()` | 用户偏好 |

### 代码摘要

```rust
let builder = ContextBuilder::new(self.memory_store.clone())
    .with_strategy(Box::new(RecentMessagesStrategy::new(10)))
    .with_strategy(Box::new(SemanticSearchStrategy::new(
        5,
        self.embedding_service.clone(),
    )))
    .with_strategy(Box::new(UserPreferencesStrategy::new()))
    .with_token_limit(4096)
    .for_user(user_id)
    .for_conversation(conversation_id)
    .with_query(question);

match builder.build().await {
    Ok(context) => { /* 格式化为字符串供 prompt 使用 */ }
    Err(e) => { /* 记录错误，返回空上下文 */ }
}
```

与外部组件的交互：

- **MemoryStore**：各策略通过 store 拉取最近消息、做语义搜索、获取偏好。
- **EmbeddingService**：由 `SemanticSearchStrategy` 调用，对用户问题做向量化。

---

## 语义检索：SemanticSearchStrategy

**文件**：`crates/memory-strategies/src/lib.rs`（SemanticSearchStrategy）

「查询是否有相关上下文」中的**语义检索**在此实现：用用户问题生成向量，再在记忆库中做相似度搜索。

### 流程

1. 若 `query` 为空或仅空白，直接返回空结果，不做检索。
2. 使用 **Embedding 服务**对 `query` 调用 `embed(query_text)` 得到向量。
3. 调用 **MemoryStore** 的 `semantic_search(&query_embedding, self.limit)`，取前 `limit` 条（当前为 5 条）。
4. 将检索到的条目格式化为消息列表，作为上下文的一部分返回。

### 与外部组件的交互

| 组件 | 交互 |
|------|------|
| **EmbeddingService** | 调用 `embed(query_text)` 生成查询向量 |
| **MemoryStore** | 调用 `semantic_search(&query_embedding, limit)` 做向量相似度检索 |
| **ContextBuilder** | 返回的 `StrategyResult::Messages(messages)` 被合并进最终 Context |

### 关键代码

```rust
// 1. 校验 query
let query_text = match query {
    Some(q) if !q.trim().is_empty() => q.trim(),
    _ => return Ok(StrategyResult::Empty),
};

// 2. 生成查询向量
let query_embedding = self.embedding_service.embed(query_text).await?;

// 3. 语义检索
let entries = store.semantic_search(&query_embedding, self.limit).await?;

// 4. 格式化为消息
let messages = entries.into_iter().map(|entry| format_message(&entry)).collect();
Ok(StrategyResult::Messages(messages))
```

---

## 策略执行：ContextBuilder.build

**文件**：`memory/src/context.rs`（约 202-257 行）

`builder.build().await` 按添加顺序执行各策略，并汇总结果：

1. 遍历所有已注册策略（RecentMessages → SemanticSearch → UserPreferences）。
2. 对每个策略调用 `build_context(store, user_id, conversation_id, query)`。
3. 将各策略返回的 `Messages` 合并到 `conversation_history`，`Preferences` 写入 `user_preferences`。
4. 计算总 token 数，组装为 `Context` 返回。

因此「先查相关上下文」的**执行顺序**由 `ContextBuilder` 的策略列表决定，语义检索在「最近消息」之后、「用户偏好」之前执行。

---

## 代码位置汇总

| 步骤 | 文件 | 位置（约） | 作用 |
|------|------|------------|------|
| 1. 决定「先查再答」 | `ai-handlers/src/sync_ai_handler.rs` | `process_normal` / `process_streaming` | 先 `build_memory_context`，再调用 AI |
| 2. 组织「查相关」 | 同上 | `build_memory_context`（约 82-114 行） | 使用 ContextBuilder + SemanticSearchStrategy 等 |
| 3. 真正「查相关」 | `crates/memory-strategies/src/lib.rs` | `SemanticSearchStrategy::build_context` | `embedding_service.embed(query)` + `store.semantic_search(...)` |
| 4. 执行策略并组装 | `memory/src/context.rs` | `ContextBuilder::build`（约 202-257 行） | 依次执行各策略，得到最终 Context |

---

## 测试用例建议

根据本文档描述的「先查相关上下文再答」流程，与现有测试对照后，建议补充以下用例。

| 场景 | 文档依据 | 当前覆盖 | 建议 |
|------|----------|----------|------|
| **SemanticSearchStrategy：query 为空或仅空白** | 语义检索流程第 1 步：「若 query 为空或仅空白，直接返回空结果，不做检索」 | memory-strategies 单测可覆盖 | 在 memory 层为 SemanticSearchStrategy 单独加单测（空/空白 query 返回 Empty、且不调用 embed/semantic_search） |
| **SemanticSearchStrategy：正常 query 调用 embed + semantic_search** | 语义检索流程第 2、3 步：embed(query) → store.semantic_search | memory-strategies 的 `strategies_test.rs` 间接覆盖（Mock EmbeddingService + MemoryStore） | 保持现状或增强断言 |
| **ContextBuilder 策略执行顺序** | 策略执行：RecentMessages → SemanticSearch → UserPreferences | 无 | 可选：在 memory 的 context 单测中，用 Mock 策略记录执行顺序，断言顺序与文档一致 |
| **先 build_memory_context 再调 AI** | 流程概览：「先查相关上下文，再调用 AI 接口」 | runner 集成测试只断言 store ≥ 2、query ≥ 1，未断言「先 query 再 store」 | 可选：在 MockMemoryStore 中记录 query 与 store 的调用顺序，断言至少一次 semantic_search 发生在第一次 add 之前（表示先检索再写回复） |
| **流式模式同样先构建上下文** | 流式模式：先 `build_memory_context`，再 `get_ai_response_stream` | 仅普通模式有 E2E（test_ai_reply_complete_flow） | 可选：增加流式路径的单元测试或 E2E，验证 process_streaming 内先调用 build_memory_context 再调用流式 AI |

**结论**：与本文档强相关、建议优先补充的是 **SemanticSearchStrategy 空/空白 query** 与 **语义检索被调用的单测**（memory 层）；其余为增强型用例，可按优先级排期。

### 断言补充建议

现有测试中与「先查上下文再答」相关的断言如下；缺失或可加强的断言见下表。

| 测试位置 | 现有断言 | 缺失/可加强的断言 | 建议 |
|----------|----------|--------------------|------|
| **runner 集成测试** `test_ai_reply_complete_flow` | `store_call_count >= 2`、`query_call_count >= 1` | 未断言「先查后答」顺序；未断言 `semantic_search` 的入参（如 limit） | 可选：在 MockMemoryStore 中记录 `add` / `semantic_search` 调用顺序，断言「至少一次 semantic_search 发生在第一次 add 之前」；若需校验 limit，可记录 `semantic_search(_, limit)` 并断言 `limit == 5` |
| **SyncAIHandler / memory** | `build_memory_context` 行为 | 可由 memory-strategies 单测或 runner 集成测试覆盖 | 纯空白 question 返回空：在 SemanticSearchStrategy 或 ContextBuilder 单测中补充 |
| **memory-strategies** | 语义检索与最近消息 | Mock 策略可断言执行顺序与 embed/semantic_search 调用 | 若需严格对应文档：用 Mock 记录 embed 与 semantic_search 调用次数 |
| **MockMemoryStore** | 仅暴露 `get_store_call_count`、`get_query_call_count` | 无调用顺序、无 semantic_search 参数记录 | 若要做「先 query 再 store」或 limit 断言：增加调用顺序缓冲区（如 `Vec<(CallKind, Option<usize>)>`），在 `add`/`semantic_search` 中 push，并提供 `get_call_order()` 或 `first_query_before_first_store()` |

**断言补充优先级**：建议优先补充 **空白 question 返回空** 的断言（实现简单、与文档一致）；「先 query 再 store」与 limit 断言视需求再补。

---

## 相关文档

- [数据流](./data-flow.md) - 消息处理与上下文在整体流程中的位置
- [ContextBuilder 设计](./context_builder_design.md) - 策略与 Context 结构
- [架构设计](./architecture.md) - RAG 模块与记忆组件的架构
- [测试方案](../TELEGRAM_BOT_TEST_PLAN.md) - 集成测试开发计划
