# Dbot 部署指南

本文档说明如何部署 Dbot Telegram 机器人并接入 Telegram 使用。

## 📋 前置要求

1. **Rust 环境**：需要安装 Rust（推荐使用 rustup）
2. **Telegram Bot Token**：从 [@BotFather](https://t.me/botfather) 获取
3. **OpenAI API Key**：用于 AI 对话功能（或使用智谱 AI）

## 🚀 快速开始

### 1. 获取 Telegram Bot Token

1. 在 Telegram 中搜索并打开 [@BotFather](https://t.me/botfather)
2. 发送 `/newbot` 命令
3. 按提示设置机器人名称和用户名
4. 获取 Bot Token（格式类似：`123456789:ABCdefGHIjklMNOpqrsTUVwxyz`）

### 2. 安装 Rust（如未安装）

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. 克隆并编译项目

```bash
# 克隆项目（如果还没有）
cd /Users/mydoczhang/GithubProject/telegram-bot

# 编译项目（Release 模式）
cargo build --release

# 编译 CLI 工具
cargo build --release --package dbot-cli
```

### 4. 配置环境变量

在项目根目录创建 `.env` 文件：

```bash
# 必需配置
BOT_TOKEN=your_bot_token_here
OPENAI_API_KEY=your_openai_api_key_here

# 数据库配置（可选，有默认值）
DATABASE_URL=file:./telegram_bot.db

# OpenAI 配置（可选，有默认值）
OPENAI_BASE_URL=https://api.openai.com/v1
LLM_MODEL=gpt-3.5-turbo
LLM_USE_STREAMING=false
LLM_THINKING_MESSAGE=正在思考...

# 记忆存储配置（可选）
MEMORY_STORE_TYPE=memory          # 选项：memory / sqlite / lance
MEMORY_SQLITE_PATH=./data/memory.db
MEMORY_LANCE_PATH=./lancedb      # 仅当 MEMORY_STORE_TYPE=lance 时需要

# Embedding 配置（可选）
EMBEDDING_PROVIDER=openai         # 选项：openai / zhipuai
BIGMODEL_API_KEY=                  # 仅当 EMBEDDING_PROVIDER=zhipuai 时需要

# RAG 参数配置（可选，有默认值）
MEMORY_RECENT_LIMIT=10            # 最近消息条数上限
MEMORY_RELEVANT_TOP_K=5           # 语义检索 Top-K
MEMORY_SEMANTIC_MIN_SCORE=0.0     # 语义检索最低相似度阈值（推荐 0.6-0.8）

# 流式回复配置（可选）
TELEGRAM_EDIT_INTERVAL_SECS=5     # 流式编辑间隔（秒）

# 系统提示词（可选，自定义机器人人设）
LLM_SYSTEM_PROMPT=你是一个友好的助手...

# 图像生成配置（已集成，使用 OpenAI DALL-E）
# 图像生成功能会自动识别包含"画"、"生成图片"等关键词的消息
# 默认使用 dall-e-3 模型，1024x1024 尺寸
```

### 5. 运行机器人

#### 方式一：使用 CLI 工具（推荐）

```bash
# 使用默认配置（从 .env 读取）
./target/release/dbot run

# 或使用命令行参数覆盖 token
./target/release/dbot run --token your_bot_token
```

#### 方式二：直接运行

```bash
# 设置环境变量后运行
export BOT_TOKEN=your_bot_token
export OPENAI_API_KEY=your_api_key
cargo run --release --package dbot-cli -- run
```

## 📝 配置说明

### 必需配置

| 环境变量 | 说明 | 示例 |
|---------|------|------|
| `BOT_TOKEN` | Telegram Bot Token | `123456789:ABCdef...` |
| `OPENAI_API_KEY` | OpenAI API Key | `sk-...` |

### 可选配置（有默认值）

| 环境变量 | 默认值 | 说明 |
|---------|--------|------|
| `DATABASE_URL` | `file:./telegram_bot.db` | SQLite 数据库路径 |
| `OPENAI_BASE_URL` | `https://api.openai.com/v1` | OpenAI API 基础 URL |
| `LLM_MODEL` | `gpt-3.5-turbo` | AI 模型名称 |
| `LLM_USE_STREAMING` | `false` | 是否使用流式回复 |
| `LLM_THINKING_MESSAGE` | `正在思考...` | 思考中提示消息 |
| `MEMORY_STORE_TYPE` | `memory` | 记忆存储类型 |
| `MEMORY_RECENT_LIMIT` | `10` | 最近消息条数上限 |
| `MEMORY_RELEVANT_TOP_K` | `5` | 语义检索 Top-K |
| `MEMORY_SEMANTIC_MIN_SCORE` | `0.0` | 语义检索最低相似度阈值 |
| `TELEGRAM_EDIT_INTERVAL_SECS` | `5` | 流式编辑间隔（秒） |

### 记忆存储类型选择

#### 1. `memory`（内存存储，默认）
- **适用场景**：开发、测试、小规模使用
- **特点**：数据不持久化，重启后丢失
- **配置**：无需额外配置

#### 2. `sqlite`（SQLite 存储）
- **适用场景**：小到中型生产环境
- **特点**：数据持久化，支持向量检索
- **配置**：
  ```bash
  MEMORY_STORE_TYPE=sqlite
  MEMORY_SQLITE_PATH=./data/memory.db
  ```

#### 3. `lance`（Lance 向量存储）
- **适用场景**：大规模生产环境，需要高性能向量检索
- **特点**：高性能向量数据库，支持大规模数据
- **配置**：
  ```bash
  MEMORY_STORE_TYPE=lance
  MEMORY_LANCE_PATH=./lancedb
  LANCE_EMBEDDING_DIM=1536  # OpenAI embedding 维度
  ```

### Embedding 提供商选择

#### OpenAI（默认）
```bash
EMBEDDING_PROVIDER=openai
OPENAI_API_KEY=your_key
```

#### 智谱 AI（ZhipuAI）
```bash
EMBEDDING_PROVIDER=zhipuai
BIGMODEL_API_KEY=your_key  # 或使用 ZHIPUAI_API_KEY
```

## 🔧 使用方式

### 启动机器人后

1. **回复机器人消息**：直接回复机器人的任意消息，机器人会自动处理并回复
2. **@ 提及机器人**：在群组或私聊中 @ 机器人并提问，例如：`@your_bot 你好`
3. **查看日志**：日志文件位于 `logs/telegram-bot.log`

### CLI 工具其他命令

```bash
# 查看帮助
./target/release/dbot --help

# 加载消息到向量数据库
./target/release/dbot load --batch-size 50

# 查询向量数据库最近 N 条记录
./target/release/dbot list-vectors --limit 100
```

## 🐳 Docker 部署（可选）

如果需要使用 Docker 部署，可以创建 `Dockerfile`：

```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --package dbot-cli

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/dbot /usr/local/bin/dbot

# 创建数据目录
RUN mkdir -p /app/data /app/logs

ENV DATABASE_URL=file:/app/data/telegram_bot.db
ENV MEMORY_SQLITE_PATH=/app/data/memory.db
ENV LOG_FILE=/app/logs/telegram-bot.log

CMD ["dbot", "run"]
```

构建和运行：

```bash
docker build -t dbot .
docker run -d \
  --name dbot \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  --env-file .env \
  dbot
```

## 🔍 故障排查

### 1. Bot Token 无效
- 检查 `BOT_TOKEN` 是否正确
- 确认从 @BotFather 获取的 token 完整

### 2. OpenAI API 调用失败
- 检查 `OPENAI_API_KEY` 是否有效
- 确认网络可以访问 `api.openai.com`
- 检查 API 配额是否充足

### 3. 数据库连接失败
- 确认 `DATABASE_URL` 路径可写
- 检查磁盘空间是否充足

### 4. 记忆存储初始化失败
- 检查 `MEMORY_SQLITE_PATH` 或 `MEMORY_LANCE_PATH` 路径可写
- 确认 `MEMORY_STORE_TYPE` 配置正确

### 5. 查看日志
```bash
# 查看实时日志
tail -f logs/telegram-bot.log

# 查看错误日志
grep ERROR logs/telegram-bot.log
```

## 📚 相关文档

- [README.md](README.md) - 项目概述
- [docs/README.md](docs/README.md) - 文档索引
- [docs/rag/README.md](docs/rag/README.md) - RAG 功能说明
- [MEMORY.md](MEMORY.md) - 记忆管理说明

## 🎨 图像生成功能

机器人已集成文生图功能，使用 OpenAI DALL-E API。当用户发送包含以下关键词的消息时，机器人会自动生成图片：

- `画` - 例如："画一只猫"
- `生成图片` - 例如："生成图片：美丽的风景"
- `生成图像`、`画图`、`/image`、`/draw`

**使用示例**：
```
用户：画一只可爱的小猫
机器人：[生成并发送图片]
```

详细使用说明请查看 [图像生成功能文档](docs/IMAGE_GENERATION.md)。

## 🎯 下一步

部署成功后，你可以：
1. 自定义系统提示词（`LLM_SYSTEM_PROMPT`）来改变机器人人设
2. 调整 RAG 参数（`MEMORY_RECENT_LIMIT`、`MEMORY_RELEVANT_TOP_K`）优化上下文质量
3. 使用 `lance` 存储类型提升大规模场景下的性能
4. 使用图像生成功能（发送包含"画"、"生成图片"等关键词的消息）
5. 查看 [docs/rag/usage.md](docs/rag/usage.md) 了解更多使用技巧
