# 配置设计

## 环境变量

```env
# 记忆配置
MEMORY_ENABLED=true
MEMORY_STORE=memory           # memory | lance | hnsw
MEMORY_MAX_CONTEXT_TOKENS=2000
MEMORY_INCLUDE_RECENT=true
MEMORY_INCLUDE_RELEVANT=true
MEMORY_RECENT_LIMIT=5
MEMORY_RELEVANT_TOP_K=3

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

### memory.recent_limit

- **类型**: usize
- **默认值**: 5
- **说明**: 最近对话记录的数量限制

### memory.relevant_top_k

- **类型**: usize
- **默认值**: 3
- **说明**: 相关历史对话的数量限制

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
