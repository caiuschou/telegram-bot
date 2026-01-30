# 成本估算

## Embedding 定价（参考）

| 服务 | 模型 | 维度 | 参考（/1K tokens） |
|------|------|------|--------------------|
| OpenAI | text-embedding-3-small | 1536 | $0.00002 |
| OpenAI | text-embedding-3-large | 3072 | $0.00013 |
| 智谱AI | embedding-2 | 1024 | 见官网 |

## 存储

- 内存+SQLite：无额外 API 成本；磁盘占用与条数、维度相关。
- Lance：同上；大规模时向量索引有内存/CPU 成本。

实际成本以各服务官网与用量为准；控制用量可调 MEMORY_RECENT_LIMIT、MEMORY_RELEVANT_TOP_K、embedding 模型与 batch 大小。
