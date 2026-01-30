# 技术选型

## 嵌入服务

| 方案 | 模型 | 维度 | 说明 |
|------|------|------|------|
| OpenAI（默认） | text-embedding-3-small / large | 1536 / 3072 | 成熟稳定 |
| 智谱AI | embedding-2 | 1024 | 中文优化，国内快 |

当前实现：embedding、openai-embedding、bigmodel-embedding；配置见 configuration.md。

## 向量存储

| 方案 | 规模 | 说明 |
|------|------|------|
| 内存+SQLite | 小规模 | 原型、开发；memory-inmemory、memory-sqlite |
| Lance | 生产、大规模 | memory-lance；见 LANCE_USAGE.md、LANCE_API_RESEARCH.md |

推荐：小规模用内存+SQLite，生产用 Lance。
