# Lance API 调研

## 概述

Lance 为开源向量/多模态数据格式与存储：高性能向量检索、Arrow 集成、可选索引（IVF_PQ、IVF_HNSW）、距离度量（Cosine/L2）。

## Rust SDK 要点

- **依赖**：lancedb、lance、arrow、datafusion（版本以 Cargo.toml 为准）。
- **连接**：`lancedb::connect(path).execute().await`；支持本地路径、S3/GCS、LanceDB Cloud。
- **表与向量**：Arrow Schema；向量列类型 `FixedSizeList<Float32>`；创建表、打开表、插入、查询见 lancedb 文档。
- **向量搜索**：`table.query().nearest_to(column, vector).limit(k).execute()`；支持 `only_if` 过滤（user_id、conversation_id 等）。
- **索引**：IVF_PQ、IVF_HNSW；nprobes、refine_factor 等参数影响精度与速度。
- **距离**：Cosine、L2；与归一化 embedding 配合常用 Cosine。

本项目实现见 memory-lance crate；使用与配置见 [LANCE_USAGE.md](LANCE_USAGE.md)、[memory/vector-search-accuracy.md](memory/vector-search-accuracy.md)。
