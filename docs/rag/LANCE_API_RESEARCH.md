# Lance API Research Documentation

## Overview

Lance is an open-source lakehouse format for multimodal AI that provides:
- High-performance vector search (100x faster than Parquet for random access)
- Full-text search capabilities (BM25)
- Zero-copy versioning
- Native multimodal data support
- SQL-based querying

## Rust SDK Overview

### Dependencies

LanceDB Rust SDK (version 0.23.1) provides:
- `lancedb` - Main database client
- `lance` - Core format and dataset operations

Key dependencies:
- `arrow ^56.2` - Apache Arrow integration
- `datafusion ^50.1` - SQL query engine
- `lance ^1.0.1` - Core Lance format

### Database Connection

```rust
use lancedb;

// Local connection
let db = lancedb::connect("data/sample-lancedb").execute().await.unwrap();

// Cloud storage (S3, GCS)
let db = lancedb::connect("s3://bucket/path").execute().await.unwrap();

// LanceDB Cloud
let db = lancedb::connect("db://dbname").execute().await.unwrap();
```

### Schema Definition

LanceDB uses Arrow schemas. Vectors are stored as `FixedSizeList<Float32>`:

```rust
use arrow_schema::{DataType, Field, Schema};

let schema = Arc::new(Schema::new(vec![
    Field::new("id", DataType::Int32, false),
    Field::new("content", DataType::Utf8, false),
    Field::new(
        "vector",
        DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            1536  // Vector dimension (e.g., OpenAI embeddings)
        ),
        false,
    ),
    Field::new("user_id", DataType::Utf8, true),
    Field::new("conversation_id", DataType::Utf8, true),
    Field::new("role", DataType::Utf8, false),
    Field::new("timestamp", DataType::Timestamp(TimeUnit::Microsecond, None), false),
    Field::new("tokens", DataType::UInt32, true),
    Field::new("importance", DataType::Float32, true),
]));
```

### Table Operations

#### Create Table

```rust
use arrow_array::{RecordBatch, RecordBatchIterator};

// Create RecordBatch stream
let batches = RecordBatchIterator::new(
    vec![RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(vec![1, 2, 3])),
            Arc::new(StringArray::from(vec!["Hello", "World", "Test"])),
            Arc::new(
                FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                    (0..3).map(|_| Some(vec![1.0; 1536])),
                    1536,
                ),
            ),
            // ... other arrays
        ],
    ).unwrap()].into_iter().map(Ok),
    schema.clone(),
);

db.create_table("memory_table", Box::new(batches))
    .mode(CreateTableMode::Overwrite)
    .execute()
    .await
    .unwrap();
```

#### Open Table

```rust
let table = db.open_table("memory_table").execute().await.unwrap();
```

#### Add Data

```rust
// Convert MemoryEntry to RecordBatch
table.add(record_batches).execute().await.unwrap();
```

### Vector Search

LanceDB provides multiple search options:

#### Semantic Search (Vector Similarity)

```rust
use lancedb::query::{ExecutableQuery, QueryBase};

let query_vector = vec![0.1f32; 1536]; // Query embedding

let results = table
    .query()
    .nearest_to(&query_vector)
    .unwrap()
    .limit(10)
    .execute()
    .await
    .unwrap()
    .try_collect::<Vec<RecordBatch>>()
    .await
    .unwrap();
```

#### Search with Filtering

```rust
let results = table
    .query()
    .nearest_to(&query_vector)
    .unwrap()
    .only_if("user_id = 'user123'")
    .limit(10)
    .execute()
    .await
    .unwrap();
```

#### Metadata Filtering (Non-vector Search)

```rust
let results = table
    .query()
    .only_if("user_id = 'user123' AND timestamp > '2024-01-01'")
    .execute()
    .await
    .unwrap();
```

### Vector Indexes

LanceDB supports efficient vector indexing for large datasets:

#### IVF_PQ (Product Quantization)

Best for: Large datasets, high dimensionality

```rust
use lancedb::index::Index;

table.create_index(&["vector"], Index::IVF_PQ {
    num_partitions: 256,  // Number of IVF partitions
    num_sub_vectors: 16,  // PQ sub-vectors
    distance_type: DistanceType::Cosine,
    ..Default::default()
})
.execute()
.await
.unwrap();
```

#### IVF_HNSW (Hierarchical Navigable Small World)

Best for: High recall requirements, moderate dataset size

```rust
table.create_index(&["vector"], Index::IVF_HNSW {
    num_partitions: 256,
    m: 16,  // Number of connections per node
    ef_construction: 100,
    distance_type: DistanceType::Cosine,
})
.execute()
.await
.unwrap();
```

### Distance Metrics

LanceDB supports multiple distance metrics:

| Metric | Description | Use Case |
|--------|-------------|-----------|
| `L2` | Euclidean distance | General-purpose similarity (default) |
| `Cosine` | Cosine similarity | Unnormalized embeddings (OpenAI) |
| `Dot` | Dot product | Normalized vectors (best performance) |
| `Hamming` | Hamming distance | Binary vectors only |

### Performance Tuning

#### Nprobes Configuration

Controls how many partitions to search:

```rust
// At query time
table
    .query()
    .nearest_to(&query_vector)
    .unwrap()
    .nprobes(20)  // Search 20 partitions (default: 10)
    .limit(10)
    .execute()
    .await
    .unwrap();
```

**Guidelines:**
- `10-20`: Good balance of recall and speed
- `50-100`: Higher recall, slower performance
- `>100`: Diminishing returns

#### Accuracy vs speed (Rust 0.23)

LanceDB Rust SDK 支持以下检索精度与速度权衡参数（`memory-lance` 通过 `LanceConfig` 暴露）：

| 手段 | API | 说明 |
|------|-----|------|
| **精确检索（暴力搜索）** | `bypass_vector_index()` | 跳过向量索引，对表中每条向量做距离计算并排序；结果最准，大数据量时最慢。无索引时本身就是 flat 搜索。 |
| **refine_factor** | `refine_factor(u32)` | 仅对 **IVF-PQ** 索引有效：先取 `limit × refine_factor` 个候选，再用完整向量重算距离并重排；值越大召回与排序越准，延迟越高。不设则使用量化距离。 |
| **nprobes** | `nprobes(usize)` | 仅对 **IVF** 类索引有效：搜索的分区数，默认 20；增大可提高召回、增加延迟。 |

小结：追求**高准确度**时可设 `use_exact_search=true`（小/中表）或调大 `refine_factor`/`nprobes`；追求**高速度**时使用默认（索引 + 默认 nprobes/不 refine）。

### Batch Operations

#### Batch Insert

```rust
let batches = vec![batch1, batch2, batch3];
table.add(batches).execute().await.unwrap();
```

#### Batch Search

```rust
let query_vectors = vec![vec1, vec2, vec3]; // Multiple queries
let results = table
    .query()
    .nearest_to(&query_vectors[0])
    .unwrap()
    .add_query_vector(&query_vectors[1])
    .unwrap()
    .add_query_vector(&query_vectors[2])
    .unwrap()
    .execute()
    .await
    .unwrap();
```

### Delete and Update Operations

#### Delete Rows

```rust
table.delete("id = '123'").execute().await.unwrap();
table.delete("timestamp < '2024-01-01'").execute().await.unwrap();
```

#### Update Rows

```rust
table.update(Set::from([("content", "new_content")]))
    .only_if("id = '123'")
    .execute()
    .await
    .unwrap();
```

### Schema Evolution

#### Add Column

```rust
use lancedb::table::NewColumnTransform;

table
    .add_columns(vec![NewColumnTransform::SqlExpressions(vec![
        ("embed_version".to_string(), "'v1.0'".to_string()),
    ])], None)
    .await
    .unwrap();
```

#### Drop Column

```rust
table.drop_columns(&["embed_version"]).await.unwrap();
```

### Versioning and Time Travel

LanceDB provides automatic versioning:

```rust
// List versions
let versions = table.list_versions().execute().await.unwrap();

// Restore to specific version
table.restore(5).execute().await.unwrap();

// Create tag
table.create_tag("backup_v1", 5).execute().await.unwrap();
```

## Integration with Memory Crate

### Data Mapping

The `MemoryEntry` type maps to LanceDB schema:

| MemoryEntry Field | LanceDB Column | Arrow Type |
|------------------|-----------------|-------------|
| `id` | `id` | `Utf8` (UUID as string) |
| `content` | `content` | `Utf8` |
| `embedding` | `vector` | `FixedSizeList<Float32>[1536]` |
| `metadata.user_id` | `user_id` | `Utf8` (nullable) |
| `metadata.conversation_id` | `conversation_id` | `Utf8` (nullable) |
| `metadata.role` | `role` | `Utf8` |
| `metadata.timestamp` | `timestamp` | `Timestamp` |
| `metadata.tokens` | `tokens` | `UInt32` (nullable) |
| `metadata.importance` | `importance` | `Float32` (nullable) |

### Implementation Strategy

1. **Connection Management**
   - Single LanceDB connection per store instance
   - Connection pooling via tokio::sync::Mutex

2. **Schema Conversion**
   - Arrow-based conversion between MemoryEntry and RecordBatch
   - Reuse schema objects for performance

3. **Indexing Strategy**
   - Create IVF_PQ index on vector column
   - Use nprobes = 10-20 for balanced performance
   - Index creation async, monitor progress

4. **Query Optimization**
   - Use prefiltering for user/conversation queries
   - Limit nprobes based on dataset size
   - Cache frequently accessed queries

5. **Batch Operations**
   - Implement batch add for multiple entries
   - Use async streams for large result sets

## Performance Benchmarks

From Lance documentation:
- Vector search on 1M vectors (128D): <1ms average response time
- Random access: 100x faster than Parquet
- Scales to billions of vectors

## Best Practices

1. **Use Cosine Distance** for OpenAI embeddings (text-embedding-ada-002, text-embedding-3-large)
2. **Create Indexes** after ingesting data for optimal performance
3. **Use Prefiltering** when filtering by metadata before vector search
4. **Batch Operations** for better performance on large datasets
5. **Monitor Index Progress** - index creation is async
6. **Appropriate nprobes** - adjust based on dataset size and recall requirements

## Future Considerations

### Full-Text Search Integration
Can combine vector search with BM25 full-text search for hybrid retrieval:

```rust
// Create FTS index on content
table.create_index(&["content"], Index::FTS).execute().await.unwrap();

// Hybrid search
table.search("user query")  // FTS
    .distance_type("cosine")
    .limit(10)
    .execute()
    .await
    .unwrap();
```

### GPU Acceleration
GPU support available for faster index building (Python SDK only)

### Cloud Integration
- S3, GCS, Azure Blob native support
- LanceDB Cloud for managed solution

## References

- LanceDB Rust SDK: https://docs.rs/lancedb/latest/lancedb/
- Lance Format: https://docs.rs/lance/latest/lance/
- Official Documentation: https://docs.lancedb.com/
- GitHub Repository: https://github.com/lance-format/lance
