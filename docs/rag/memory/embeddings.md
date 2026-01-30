# 嵌入服务

## EmbeddingService Trait

- **embed**(text) → Vec<f32>
- **embed_batch**(texts) → Vec<Vec<f32>>

## 实现

- **OpenAIEmbedding**（openai-embedding）：text-embedding-3-small / large 等；需 API key、有成本。
- **BigmodelEmbedding**（bigmodel-embedding）：智谱 embedding-2 等。

配置见 docs/rag/configuration.md；维度与模型选择见 [vector-search-accuracy.md](vector-search-accuracy.md)。
