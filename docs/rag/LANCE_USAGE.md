# Lance 向量存储使用指南

## 概述

Lance 是一个高性能向量数据库，专为 AI 应用设计。它提供了比内存存储更好的性能和持久化能力。

## 安装要求

### Protocol Buffers Compiler (protoc)

Lance 需要 `protoc` 来编译其原生依赖：

**Linux (Ubuntu/Debian)**:
```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler
```

**WSL**:
```bash
sudo apt-get update
sudo apt-get install -y protobuf-compiler
```

**macOS**:
```bash
brew install protobuf
```

验证安装:
```bash
protoc --version
# 期望输出: libprotoc 28.x 或更高
```

## 配置

在 `.env` 文件中添加以下配置：

```bash
# 使用 Lance 向量存储
MEMORY_STORE_TYPE=lance

# LanceDB 数据库路径（默认: ./data/lancedb）
LANCE_DB_PATH=./data/lancedb
```

## 存储类型对比

| 类型 | 优点 | 缺点 | 适用场景 |
|------|------|------|----------|
| **memory** | 无需配置，启动快 | 重启后数据丢失 | 开发测试 |
| **sqlite** | 持久化存储 | 大数据量性能一般 | 小规模生产 |
| **lance** | 高性能，可扩展 | 需要 protoc | 生产环境推荐 |

## 首次使用

1. **安装 protoc**（见上方）

2. **配置环境变量**:
```bash
cp .env.example .env
# 编辑 .env，设置 MEMORY_STORE_TYPE=lance
```

3. **运行 bot**:
```bash
cargo run --bin dbot
```

LanceDB 会自动在指定路径创建数据库文件和表结构。

## 数据迁移

如果需要从其他存储迁移到 Lance：

```rust
use memory::migration::sqlite_to_lance;

// 从 SQLite 迁移到 Lance
let count = sqlite_to_lance("./data/memory.db", "./data/lancedb").await?;
println!("迁移了 {} 条记录", count);
```

## 性能调优

### 创建索引

对于大量数据，建议创建索引以加速查询：

```rust
use memory::LanceVectorStore;

let store = LanceVectorStore::new("./data/lancedb").await?;

// 创建向量索引（可选）
store.create_index(memory::LanceIndexType::Auto).await?;
```

### 配置选项

Lance 支持多种配置选项：

```rust
use memory::{LanceConfig, DistanceType};

let config = LanceConfig {
    db_path: "./data/lancedb".to_string(),
    table_name: "memories".to_string(),
    embedding_dim: 1536,  // OpenAI text-embedding-ada-002
    distance_type: DistanceType::Cosine,
};

let store = LanceVectorStore::with_config(config).await?;
```

## 故障排查

### 编译错误

如果看到 `error: failed to run custom build command for lance-encoding`:
- 确认 `protoc` 已安装并在 PATH 中
- 尝试设置环境变量: `export PROTOC=/usr/bin/protoc`

### 运行时错误

如果启动时遇到连接数据库错误：
- 检查 `LANCE_DB_PATH` 目录是否存在
- 确保程序有读写权限

## 参考文档

- [Lance 官方文档](https://lancedb.github.io/lance/)
- [memory crate 文档](../memory/LANCE_INTEGRATION.md)
