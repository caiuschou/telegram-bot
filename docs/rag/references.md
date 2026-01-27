# 参考资料

## 向量数据库

### Lance
- **官网**: https://lancedb.com/
- **GitHub**: https://github.com/lance-format/lance
- **文档**: https://lancedb.github.io/lance/
- **推荐理由**: 原生Rust实现，嵌入式设计，支持向量索引和元数据过滤

### HNSW-rs
- **GitHub**: https://github.com/jorgecarleitao/arrow2
- **说明**: 高效的HNSW索引算法实现
- **适用场景**: 中等规模数据，需要高性能检索

### redb
- **GitHub**: https://github.com/cberner/redb
- **说明**: 嵌入式键值数据库，纯Rust实现
- **适用场景**: 需要轻量级持久化存储

### 其他向量数据库
- **Qdrant**: https://qdrant.tech/ - 支持分布式部署
- **Milvus**: https://milvus.io/ - 开源向量数据库
- **Chroma**: https://www.trychroma.com/ - 轻量级向量数据库

## 嵌入服务

### OpenAI API
- **Embeddings API**: https://platform.openai.com/docs/guides/embeddings
- **Chat Completions API**: https://platform.openai.com/docs/api-reference/chat
- **定价**: https://platform.openai.com/pricing
- **最佳实践**: https://platform.openai.com/docs/guides/embeddings/embedding-models

### 智谱AI API
- **官网**: https://open.bigmodel.cn/ (需要JavaScript访问)
- **GitHub**: https://github.com/MetaGLM/zhipuai-sdk-python-v4
- **新SDK**: https://github.com/zai-org/z-ai-sdk-python (推荐使用)
- **Python SDK**: `pip install zhipuai`
- **定价**: https://open.bigmodel.cn/ (需查询官网)
- **说明**: 国产大模型，中文支持优秀，国内访问更快

## RAG相关

### OpenAI API
- **Embeddings API**: https://platform.openai.com/docs/guides/embeddings
- **Chat Completions API**: https://platform.openai.com/docs/api-reference/chat
- **定价**: https://platform.openai.com/pricing
- **最佳实践**: https://platform.openai.com/docs/guides/embeddings/embedding-models
- **Embeddings API**: https://platform.openai.com/docs/guides/embeddings
- **Chat Completions API**: https://platform.openai.com/docs/api-reference/chat
- **定价**: https://platform.openai.com/pricing
- **最佳实践**: https://platform.openai.com/docs/guides/embeddings/embedding-models

### 学术论文
- **Memory-Augmented Language Models**: https://arxiv.org/abs/2002.08916
- **ReAct: Synergizing Reasoning and Acting**: https://arxiv.org/abs/2210.03629
- **Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks**: https://arxiv.org/abs/2005.11401

### RAG框架
- **LangChain**: https://python.langchain.com/ - 流行的RAG框架（Python）
- **LlamaIndex**: https://www.llamaindex.ai/ - 数据框架，支持RAG

## Rust生态

### Telegram Bot
- **Teloxide**: https://docs.rs/teloxide/latest/teloxide/ - Telegram Bot框架
- **frand**: https://github.com/teloxide/teloxide - 示例和最佳实践

### OpenAI客户端
- **async-openai**: https://github.com/64bit/async-openai - 异步OpenAI客户端
- **openai-api-rs**: https://github.com/64bit/openai-api-rs - 同步版本

### 智谱AI客户端
- **zhipuai-sdk-python-v4**: https://github.com/MetaGLM/zhipuai-sdk-python-v4 - Python SDK
- **z-ai-sdk-python**: https://github.com/zai-org/z-ai-sdk-python - 新版Python SDK（推荐）
- **注意**: 智谱AI目前主要提供Python SDK，Rust项目可通过HTTP API直接调用或使用Python服务

### 数据库
- **SQLx**: https://docs.rs/sqlx/latest/sqlx/ - 异步SQL工具包
- **Diesel**: https://diesel.rs/ - ORM和查询构建器

### 其他
- **Tokio**: https://tokio.rs/ - 异步运行时
- **Serde**: https://serde.rs/ - 序列化/反序列化
- ** anyhow**: https://docs.rs/anyhow/latest/anyhow/ - 错误处理

## 向量相似度算法

### 余弦相似度
```
cosine_similarity(a, b) = (a · b) / (||a|| * ||b||)
```

### 欧氏距离
```
euclidean_distance(a, b) = sqrt(sum((a_i - b_i)^2))
```

### 点积
```
dot_product(a, b) = sum(a_i * b_i)
```

## 向量索引算法

### HNSW (Hierarchical Navigable Small World)
- **论文**: https://arxiv.org/abs/1603.09320
- **特点**: 高效的近似最近邻搜索
- **适用**: 中等规模数据，需要低延迟

### IVF_PQ (Inverted File with Product Quantization)
- **特点**: 平衡搜索精度和内存占用
- **适用**: 大规模数据

### ScaNN (Scalable Nearest Neighbors)
- **论文**: https://arxiv.org/abs/1908.10396
- **特点**: 高吞吐量，低延迟
- **适用**: 大规模实时检索

## 相关工具

### 向量可视化
- **TensorBoard**: https://www.tensorflow.org/tensorboard - 支持向量降维可视化
- **t-SNE**: https://lvdmaaten.github.io/tsne/ - 流形学习降维算法

### 性能测试
- **Criterion**: https://bheisler.github.io/criterion.rs/book/index.html - Rust基准测试工具
- **criterion.rs**: https://github.com/bheisler/criterion.rs

### 文档生成
- **Rustdoc**: https://doc.rust-lang.org/rustdoc/ - Rust官方文档工具
- **mdBook**: https://rust-lang.github.io/mdBook/ - 书籍格式文档

## 社区资源

### 论坛和讨论
- **Rust用户论坛**: https://users.rust-lang.org/
- **Stack Overflow - Rust**: https://stackoverflow.com/questions/tagged/rust
- **Reddit r/rust**: https://www.reddit.com/r/rust/

### 学习资源
- **The Rust Book**: https://doc.rust-lang.org/book/
- **Rust by Example**: https://doc.rust-lang.org/rust-by-example/
- **Rustlings**: https://rustlings.cool/

### AI/RAG相关
- **Pinecone Learn**: https://www.pinecone.io/learn/
- **Weaviate Blog**: https://weaviate.io/blog
- **Qdrant Blog**: https://qdrant.tech/articles/

## 依赖版本参考

```toml
[dependencies]
# 向量数据库
lance = "2.0"
lance-arrow = "2.0"
hnsw = "0.12"

# OpenAI
async-openai = "0.20"

# 异步运行时
tokio = { version = "1.35", features = ["full"] }

# 数据库
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite"] }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 错误处理
anyhow = "1.0"
thiserror = "1.0"

# 日志
tracing = "0.1"
tracing-subscriber = "0.3"

# 工具
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

## 部署相关

### Docker
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/dbot /usr/local/bin/dbot
CMD ["dbot"]
```

### 环境变量最佳实践
- 使用 `.env` 文件管理本地配置
- 生产环境使用密钥管理服务（如AWS Secrets Manager）
- 敏感信息不要提交到版本控制

### 监控和日志
- 使用 Prometheus + Grafana 监控性能
- 使用 ELK Stack 或 Loki 收集日志
- 记录关键指标：API调用次数、响应时间、错误率
