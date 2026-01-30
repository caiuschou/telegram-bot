# 配置设计

## 环境变量（设计参考）

当前 Telegram Bot 实际使用的 RAG 相关项见下表；其余为设计或可选。

```env
# 记忆
MEMORY_RECENT_LIMIT=10          # 近期条数，默认 10
MEMORY_RELEVANT_TOP_K=5        # 语义 Top-K，默认 5
MEMORY_SEMANTIC_MIN_SCORE=0.0  # 相似度阈值，0=不过滤；推荐 0.6–0.8
MEMORY_RECENT_USE_SQLITE=0     # 1=近期来自 SQLite
MEMORY_STORE_TYPE=memory       # memory | sqlite | lance

# 嵌入
EMBEDDING_PROVIDER=openai      # openai | zhipuai
EMBEDDING_MODEL=text-embedding-3-small
LANCE_DB_PATH=./data/lancedb  # Lance 时使用
```

## Telegram Bot 实际环境变量（BotConfig）

| 变量 | 含义 | 默认 | 推荐 |
|------|------|------|------|
| MEMORY_RECENT_LIMIT | 近期消息条数 | 10 | 5–20 |
| MEMORY_RELEVANT_TOP_K | 语义 Top-K | 5 | 3–10 |
| MEMORY_SEMANTIC_MIN_SCORE | 语义最低分数 | 0.0 | 0.6–0.8（或 0 不过滤） |

详见 `telegram-bot/src/config.rs`；向量准确度与成本见 [memory/vector-search-accuracy.md](memory/vector-search-accuracy.md)。
