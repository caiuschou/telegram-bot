# RAG (Retrieval-Augmented Generation) 集成方案

> 本文档已迁移到新的文档结构，请访问 [docs/rag/](./rag/) 查看完整内容。

## 快速导航

- **[概述](./rag/README.md)** - 概述、核心目标、模块结构概览
- **[架构设计](./rag/architecture.md)** - 模块结构、核心组件详细说明
- **[技术选型](./rag/technical-selection.md)** - 向量数据库方案对比和选择
- **[数据流](./rag/data-flow.md)** - 消息处理流程和上下文构建
- **[配置](./rag/configuration.md)** - 环境变量和配置文件
- **[实现计划](./rag/implementation.md)** - 分阶段开发路线图
- **[使用示例](./rag/usage.md)** - 代码示例和最佳实践
- **[测试策略](./rag/testing.md)** - 单元测试、集成测试、性能测试
- **[成本估算](./rag/cost.md)** - API成本和存储成本分析
- **[未来扩展](./rag/future.md)** - 功能扩展方向
- **[参考资料](./rag/references.md)** - 相关链接和文档

## 概述

为 dbot 项目集成 RAG 能力，用于机器人的**对话记忆管理**、**上下文构建**以及**回复前检索相关历史对话**。

## 核心目标

1. **长期记忆**：将历史对话向量化存储，支持跨会话的记忆检索
2. **短期上下文**：管理当前对话窗口，确保上下文连贯性
3. **智能检索**：根据当前问题检索相关的历史对话片段
4. **个性化记忆**：存储和检索用户偏好、重要信息

## 快速开始

```bash
# 查看完整文档
ls docs/rag/

# 阅读架构设计
cat docs/rag/architecture.md

# 查看实现计划
cat docs/rag/implementation.md
```
