# ContextBuilder 设计

## 概述

ContextBuilder 负责从 MemoryStore 检索并组织信息，构建 AI 对话的上下文。

## 目标

- 多策略构建（近期、语义相关、用户偏好）
- Token 窗口管理
- 语义相关历史检索
- 用户相关信息与偏好

## 架构

- **ContextBuilder**：协调策略与 token 管理；持有 store、strategies、token_limit。
- **策略**：RecentMessagesStrategy（最近 N 条）、SemanticSearchStrategy（语义 Top-K）、UserPreferencesStrategy（用户偏好）。
- **TokenWindowManager**：估算 token、超限截断、优先级（近期/重要优先）。
- **Context**：system_message、conversation_history、user_preferences、metadata。

实现见 memory/src/context/builder.rs、memory-strategies。
