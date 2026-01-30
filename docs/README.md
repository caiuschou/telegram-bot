# 文档索引

项目文档入口与导航。每个主题「索引 + 子页」放在本目录或对应子目录。

## 根目录文档

| 文档 | 说明 |
|------|------|
| [CRATES.md](CRATES.md) | Crate 与文件索引，各 crate 职责与路径 |
| [api-endpoints.md](api-endpoints.md) | API 端点说明（智谱等兼容格式） |
| [LOGGING.md](LOGGING.md) | 日志文件、格式、级别与轮转 |
| [RUST_TELEGRAM_BOT_GUIDE.md](RUST_TELEGRAM_BOT_GUIDE.md) | Rust Telegram Bot 开发完整指南 |
| [AI_MENTION_REPLY.md](AI_MENTION_REPLY.md) | Bot 被 @ 时 AI 回复方案与实施清单 |

## 功能与方案

| 文档 | 说明 |
|------|------|
| [RAG_SOLUTION.md](RAG_SOLUTION.md) | RAG 集成方案（跳转到 [rag/](rag/)） |
| [rag/](rag/README.md) | RAG：架构、配置、数据流、实现与使用 |
| [db_query_result.md](db_query_result.md) | 数据库查询结果说明（memory_entries / messages） |
| [PERSISTENCE_CHECK.md](PERSISTENCE_CHECK.md) | 消息持久化功能检查报告 |

## CLI

| 文档 | 说明 |
|------|------|
| [dbot-cli-vector-query-plan.md](dbot-cli-vector-query-plan.md) | CLI 向量库查询方案（list-vectors） |
| [dbot-cli-code-review.md](dbot-cli-code-review.md) | dbot-cli 代码审核报告 |

## 测试与计划

| 文档 | 说明 |
|------|------|
| [TELEGRAM_BOT_TEST_PLAN.md](TELEGRAM_BOT_TEST_PLAN.md) | 集成测试方案与环境配置 |
| [TEST_COVERAGE.md](TEST_COVERAGE.md) | 测试覆盖说明 |
| [rust-telegram-bot-plan.md](rust-telegram-bot-plan.md) | Rust Telegram Bot 开发计划 |

## 重构相关

| 文档 | 说明 |
|------|------|
| [refactoring/](refactoring/README.md) | 重构文档索引（框架抽离、Bot/AI 拆分、HandlerChain 等） |

## 项目根目录

- [README.md](../README.md) - 项目说明与快速开始
- [SETUP.md](../SETUP.md) - 环境与运行配置
- [MEMORY.md](../MEMORY.md) - 记忆管理概述与文档链接
- [CHANGELOGS.md](../CHANGELOGS.md) - 变更记录
