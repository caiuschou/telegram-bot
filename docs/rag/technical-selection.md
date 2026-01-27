# 技术选型

## 嵌入服务方案

### 方案A：OpenAI Embeddings（默认）

**选择理由**：
- 成熟的嵌入服务，稳定可靠
- 支持多种嵌入模型（text-embedding-3-small、text-embedding-3-large）
- 向量维度：1536维（small）或3072维（large）
- 优秀的语义理解能力

**模型选择**：
- **text-embedding-3-small**: 1536维，成本最低（$0.02/1M tokens）
- **text-embedding-3-large**: 3072维，性能更好（$0.13/1M tokens）
- **text-embedding-ada-002**: 1536维，旧版模型（$0.10/1M tokens）

### 方案B：智谱AI Embeddings

**选择理由**：
- 国产大模型，中文支持优秀
- 成本较低（具体价格需查询官网）
- 向量维度：1024维（embedding-2模型）
- 国内访问速度更快
- 提供 Python SDK：`pip install zhipuai`

**模型选择**：
- **embedding-2**: 1024维，通用语义嵌入

**使用示例**：
```python
from zhipuai import ZhipuAI

client = ZhipuAI(api_key="your-api-key")

response = client.embeddings.create(
    model="embedding-2",
    input=["文本1", "文本2"],
    encoding_format="float"
)
```

**依赖**：
```toml
# Python SDK（如果使用Python服务）
# pip install zhipuai
```

**配置示例**：
```env
EMBEDDING_PROVIDER=zhipuai
EMBEDDING_MODEL=embedding-2
ZHIPUAI_API_KEY=your-api-key
ZHIPUAI_BASE_URL=https://open.bigmodel.cn/api/paas/v4/
```

## 嵌入式向量数据库方案

### 方案A：Lance（推荐）

**选择理由**：
- 原生Rust实现，性能优异
- 嵌入式设计，文件存储，无需额外服务
- 支持向量索引（IVF_PQ、HNSW）
- 支持元数据过滤（按用户、时间范围）
- 支持批量操作和事务

**依赖**：
```toml
lance = "2.0"
lance-arrow = "2.0"
```

**使用示例**：
```rust
use lance::LanceError;
use arrow_array::{Float32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};

// 创建向量存储
let schema = Schema::new(vec![
    Field::new("id", DataType::Utf8, false),
    Field::new("user_id", DataType::Int64, false),
    Field::new("content", DataType::Utf8, false),
    Field::new("vector", DataType::FixedSizeList(
        Arc::new(Field::new("item", DataType::Float32, false)),
        1536,
    ), false),
    Field::new("timestamp", DataType::Int64, false),
]);

let dataset = lance::dataset::Dataset::write(&batch, "./memories.lance").await?;
```

### 方案B：内存向量存储 + SQLite

**选择理由**：
- 最简单的实现，无额外依赖
- 利用现有的SQLite存储原始数据
- 适合小规模（<10000条）记忆
- 快速原型开发

**实现方式**：
```rust
// SQLite存储原始数据 + 元数据
// 内存中存储向量索引
pub struct InMemoryVectorStore {
    entries: Vec<(MemoryEntry, Vec<f32>)>,
    user_index: HashMap<i64, Vec<usize>>,
}

impl InMemoryVectorStore {
    pub async fn search(&self, query: &Vec<f32>, top_k: usize) -> Result<Vec<MemoryEntry>> {
        let mut results: Vec<(usize, f32)> = self.entries
            .iter()
            .map(|(idx, (_, embedding))| {
                (idx, cosine_similarity(query, embedding))
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        Ok(results.iter().take(top_k)
            .map(|(idx, _)| self.entries[*idx].0.clone())
            .collect())
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot_product / (norm_a * norm_b)
}
```

### 方案C：HNSW索引

**选择理由**：
- 高效的近似最近邻搜索
- 内存占用小
- 适合中等规模（10000-100000条）
- 有成熟的Rust实现

**依赖**：
```toml
hnsw = "0.12"
```

## 存储策略

### 双层存储（推荐方案A）

1. **Lance**：存储向量化记忆 + 原始数据（统一存储）
2. 支持向量索引和SQL查询

### 三层存储（方案B）

1. **SQLite**：存储原始对话记录（storage模块已有）
2. **内存**：存储向量索引
3. **磁盘**：定期持久化向量索引（可选）

### 数据流转

```
用户消息 → 向量化 → Lance存储（向量+元数据）
         ↓
     检索相关记忆 → 构建上下文 → AI生成 → 保存回复
```

## 方案对比

| 特性 | Lance | 内存+SQLite | HNSW |
|------|-------|-------------|------|
| 性能 | ⭐⭐⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐⭐ |
| 易用性 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| 规模 | 100万+ | <1万 | 10万+ |
| 依赖 | Lance | 无 | HNSW |
| 持久化 | ✅ | 需额外实现 | ✅ |
| 元数据过滤 | ✅ | 需手动实现 | ❌ |

## 推荐方案

- **小规模/原型**：内存+SQLite
- **生产环境**：Lance
