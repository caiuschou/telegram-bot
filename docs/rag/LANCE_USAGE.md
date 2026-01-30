# Lance 向量存储使用指南

## 概述

Lance 为高性能向量库，适合生产环境；需 `protoc`。

## 安装 protoc

- **Linux (Ubuntu/Debian/WSL)**：`sudo apt-get install -y protobuf-compiler`
- **macOS**：`brew install protobuf`
- 验证：`protoc --version`（建议 28.x+）

## 配置

```env
MEMORY_STORE_TYPE=lance
LANCE_DB_PATH=./data/lancedb
```

## 存储类型对比

| 类型 | 优点 | 适用 |
|------|------|------|
| memory | 无需配置 | 开发测试 |
| sqlite | 持久化 | 小规模 |
| lance | 高性能、可扩展 | 生产推荐 |

## 首次使用

1. 安装 protoc；2. 设置 MEMORY_STORE_TYPE=lance、LANCE_DB_PATH；3. 运行 bot（如 `cargo run --package dbot-cli`）。Lance 会自动建库与表。

## 数据迁移与调优

- 从 SQLite 迁移：见 memory/migration、dbot-cli `load` 子命令（从 storage 加载到 Lance）。
- 索引与精度：见 [memory/vector-search-accuracy.md](memory/vector-search-accuracy.md)、[LANCE_API_RESEARCH.md](LANCE_API_RESEARCH.md)。
