# AI 回答前上下文检索

## 概述

AI 回复前先根据当前问题从记忆库取相关上下文（近期、语义相关、用户偏好），再与问题一起交给模型。本文档说明代码位置与调用关系。

## 流程

用户 @ 提及 → SyncAIHandler.handle() → process_normal / process_streaming → build_messages_for_ai() 内先调用 **build_memory_context()** → ContextBuilder 依次执行 RecentMessagesStrategy、SemanticSearchStrategy、UserPreferencesStrategy → context.to_messages(question) → LlmClient 生成回复。

## 代码位置

- **入口**：ai-handlers `SyncAIHandler::handle` → process_normal / process_streaming；两者均先 build_memory_context。
- **上下文构建**：SyncAIHandler::build_memory_context（ContextBuilder + 策略配置）；ContextBuilder::build 执行各策略。
- **语义检索**：SemanticSearchStrategy（embed query + store.semantic_search，可选分数过滤）。

详见 ai-handlers/src/sync_ai_handler.rs、memory-strategies、memory/src/context/builder.rs。
