# 配置设计

## 环境变量

以下为设计参考；**当前 Telegram Bot 实际从环境变量读取的 RAG 相关项**见「Telegram Bot 实现的环境变量」小节。

```env
# 记忆配置
MEMORY_ENABLED=true
MEMORY_STORE=memory           # memory | lance | hnsw
MEMORY_MAX_CONTEXT_TOKENS=2000
MEMORY_INCLUDE_RECENT=true
MEMORY_INCLUDE_RELEVANT=true
MEMORY_RECENT_LIMIT=10        # 近期消息条数，默认 10（与 BotConfig 一致）
MEMORY_RELEVANT_TOP_K=5       # 语义检索 Top-K，默认 5（与 BotConfig 一致）
MEMORY_SEMANTIC_MIN_SCORE=0.0 # 语义检索最低相似度阈值，默认 0.0（不过滤）；推荐 0.6–0.8

# 嵌入服务配置
EMBEDDING_PROVIDER=openai     # openai | zhipuai
EMBEDDING_MODEL=text-embedding-3-small
EMBEDDING_BATCH_SIZE=10

# 智谱AI配置（如果使用智谱AI）
ZHIPUAI_API_KEY=your-api-key
ZHIPUAI_BASE_URL=https://open.bigmodel.cn/api/paas/v4/

# Lance配置（如果使用Lance）
LANCE_DATA_PATH=./data/memories.lance
```

## 配置文件

```toml
[embedding]
provider = "openai"           # openai | zhipuai
model = "text-embedding-3-small"
batch_size = 10
dimensions = 1536

[zhipuai]
api_key = "your-api-key"
base_url = "https://open.bigmodel.cn/api/paas/v4/"
```

## 配置项说明

### memory.enabled

- **类型**: bool
- **默认值**: true
- **说明**: 是否启用记忆功能

### memory.store

- **类型**: string
- **可选值**: "memory", "lance", "hnsw"
- **默认值**: "memory"
- **说明**: 记忆存储方案选择
  - `memory`: 内存+SQLite（适合小规模）
  - `lance`: Lance向量数据库（适合生产）
  - `hnsw`: HNSW索引（适合中等规模）

### memory.max_context_tokens

- **类型**: usize
- **默认值**: 2000
- **说明**: 上下文窗口最大token数，超过会自动截断

### memory.include_recent

- **类型**: bool
- **默认值**: true
- **说明**: 是否包含最近的对话记录

### memory.include_relevant

- **类型**: bool
- **默认值**: true
- **说明**: 是否检索相关的历史对话

### memory.recent_limit / MEMORY_RECENT_LIMIT

- **类型**: 正整数（u32）
- **默认值**: 10（当前 Telegram Bot `BotConfig` 默认）
- **说明**: 近期对话记录条数上限，用于 RAG 上下文的 `RecentMessagesStrategy`。推荐范围：5–20；过大会增加 token 消耗与延迟。

### memory.relevant_top_k / MEMORY_RELEVANT_TOP_K

- **类型**: 正整数（u32）
- **默认值**: 5（当前 Telegram Bot `BotConfig` 默认）
- **说明**: 语义检索返回条数（Top-K），用于 RAG 上下文的 `SemanticSearchStrategy`。推荐范围：3–10；过大会引入更多无关上下文。

### memory.semantic_min_score / MEMORY_SEMANTIC_MIN_SCORE

- **类型**: 浮点数（f32）
- **默认值**: 0.0（当前 Telegram Bot `BotConfig` 默认，表示不过滤）
- **说明**: 语义检索最低相似度阈值；低于此分数的条目不进入上下文。推荐范围：0.6–0.8 可减少无关上下文；0.0 表示不过滤，保持原有行为。详见 [向量搜索准确度](memory/vector-search-accuracy.md)。

### embedding.provider

- **类型**: string
- **可选值**: "openai", "zhipuai"
- **默认值**: "openai"
- **说明**: 嵌入服务提供商选择
  - `openai`: OpenAI Embeddings API
  - `zhipuai`: 智谱AI Embeddings API（中文优化）

### embedding.model

- **类型**: string
- **默认值**: "text-embedding-3-small"
- **OpenAI可选值**:
  - "text-embedding-3-small": 1536维，成本最低
  - "text-embedding-3-large": 3072维，性能更好
  - "text-embedding-ada-002": 1536维，旧版模型
- **智谱AI可选值**:
  - "embedding-2": 1024维，通用语义嵌入
- **说明**: 嵌入模型选择（根据provider选择对应模型）

### embedding.batch_size

- **类型**: usize
- **默认值**: 10
- **说明**: 批量向量化的大小

### embedding.dimensions

- **类型**: usize
- **默认值**: 1536（OpenAI）或 1024（智谱AI）
- **说明**: 向量维度（根据模型自动设置）

### zhipuai.api_key

- **类型**: string
- **默认值**: 从环境变量 `ZHIPUAI_API_KEY` 读取
- **说明**: 智谱AI API密钥

### zhipuai.base_url

- **类型**: string
- **默认值**: "https://open.bigmodel.cn/api/paas/v4/"
- **说明**: 智谱AI API基础URL

### lance.data_path

- **类型**: string
- **默认值**: "./data/memories.lance"
- **说明**: Lance数据库文件路径

---

## Telegram Bot 实现的环境变量（BotConfig）

当前 `telegram-bot` 从环境变量读取的 RAG 相关配置如下，与阶段 1「配置接入」一致。

| 环境变量 | 含义 | 默认值 | 推荐范围 |
|----------|------|--------|----------|
| `MEMORY_RECENT_LIMIT` | 近期消息条数上限（RecentMessagesStrategy） | 10 | 5–20 |
| `MEMORY_RELEVANT_TOP_K` | 语义检索 Top-K（SemanticSearchStrategy） | 5 | 3–10 |

未设置时使用默认值；设置无效数字时回退为默认值。详见 `telegram-bot/src/config.rs`。
