# RAG (Retrieval-Augmented Generation) 集成方案

## 概述

为 dbot 项目集成 RAG 能力，用于机器人的**对话记忆管理**、**上下文构建**以及**回复前检索相关历史对话**。

## 核心目标

1. **长期记忆**：将历史对话向量化存储，支持跨会话的记忆检索
2. **短期上下文**：管理当前对话窗口，确保上下文连贯性
3. **智能检索**：根据当前问题检索相关的历史对话片段
4. **个性化记忆**：存储和检索用户偏好、重要信息

## 快速开始

- [架构设计](./architecture.md) - 模块结构和核心组件
- [技术选型](./technical-selection.md) - 向量数据库方案对比和选择
- [数据流](./data-flow.md) - 消息处理流程和上下文构建
- [Context 发送方案](./context-sending-scheme.md) - context 以 JSON 消息形式发送到 OpenAI 的约定与实现
- [回答前上下文检索](./context-retrieval-before-reply.md) - AI 回答前查询相关上下文的代码位置与流程
- [配置](./configuration.md) - 环境变量和配置文件
- [实现计划](./implementation.md) - 分阶段开发路线图
- [向量搜索准确度优化计划](./vector-search-accuracy-plan.md) - 语义检索准确度优化开发任务表
- [使用示例](./usage.md) - 代码示例和最佳实践
- [测试策略](./testing.md) - 单元测试、集成测试、性能测试
- [成本估算](./cost.md) - API成本和存储成本分析
- [未来扩展](./future.md) - 功能扩展方向
- [参考资料](./references.md) - 相关链接和文档

## 模块结构

```
memory/                       # 新增：记忆管理模块
├── src/
│   ├── lib.rs               # 记忆trait和核心实现
│   ├── conversation.rs      # 对话记忆（ConversationMemory）
│   ├── embedding.rs         # 嵌入服务（OpenAI）
│   ├── retrieval.rs         # 检索器（语义搜索）
│   ├── context.rs           # 上下文构建器
│   └── types.rs             # 核心类型定义
└── Cargo.toml

telegram-bot-ai/             # 扩展：AI集成模块
├── src/
│   └── lib.rs               # TelegramBotAI
└── Cargo.toml

bot-runtime/                 # 扩展：运行时
├── src/
│   ├── memory_middleware.rs # 记忆中间件（新增）
│   └── ...
└── Cargo.toml
```

## 核心组件概览

### 1. memory 模块

- 对话消息的存储和向量化
- 用户偏好和重要信息的记忆
- 语义检索相关历史
- 上下文窗口管理

### 2. 记忆中间件

- 自动保存对话到记忆库
- 在处理消息前检索相关上下文
- 注入上下文到AI提示词

### 3. AI Bot 增强

- 接收记忆中间件提供的上下文
- 构建完整的AI提示词
- 生成回复并保存到记忆

## 方案选择

| 方案 | 适用场景 | 开发周期 | 部署复杂度 |
|------|---------|---------|-----------|
| 内存+SQLite | 原型、小规模 | 1-2天 | 简单 |
| Lance | 生产、大规模 | 3-5天 | 简单 |
| HNSW | 中等规模 | 2-3天 | 中等 |

**推荐**：小规模使用内存+SQLite，生产环境使用Lance。

## 相关文档

- [architecture.md](./architecture.md) - 详细架构设计
- [technical-selection.md](./technical-selection.md) - 技术方案对比
- [implementation.md](./implementation.md) - 实现路线图
