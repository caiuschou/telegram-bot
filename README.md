# Telegram Bot

An intelligent Telegram chatbot written in Rust, featuring LLM conversations, RAG memory retrieval, and streaming responses.

## Features

- **LLM Conversations**: Supports OpenAI API-compatible large language models (GPT, GLM, etc.)
- **Streaming Responses**: Real-time streaming output for better user experience
- **RAG Memory System**: Semantic search-based context memory for smarter interactions
- **Multiple Storage Backends**: Supports in-memory, SQLite, and LanceDB storage
- **Multiple Embedding Services**: Supports OpenAI and Zhipu AI (BigModel) embeddings
- **Modular Architecture**: Clean Workspace structure, easy to extend and maintain

## Project Structure

```
telegram-bot/
├── telegram-bot/       # Bot framework (core, handler chain, telegram adapter, handlers, config, CLI)
│   └── src/
│       ├── embedding/      # Embedding trait + OpenAI/BigModel implementations
│       ├── memory_core/    # Memory core types and MemoryStore trait
│       └── memory_strategies/  # Context strategies (recent, semantic, user preferences)
├── telegram-llm-bot/   # CLI entry point + LLM integration (SyncLLMHandler, run_bot_with_llm)
│   └── (llm_handlers module in src)
├── storage/            # Message persistence storage (SQLite)
├── memory/             # Memory system and context building
├── crates/
│   ├── llm/
│   │   ├── openai-client/      # OpenAI client
│   │   └── telegram-bot-llm/   # Telegram LLM integration
│   ├── llm-client/             # LLM client abstraction
│   ├── embedding/
│   │   ├── embedding/          # Embedding service trait
│   │   ├── openai-embedding/   # OpenAI embedding implementation
│   │   └── bigmodel-embedding/ # Zhipu AI embedding implementation
│   ├── memory/
│   │   ├── memory-sqlite/      # SQLite storage
│   │   └── memory-lance/       # LanceDB vector storage
│   └── prompt/                 # Prompt templates
```

## Quick Start

### Prerequisites

- Rust 1.70+
- Telegram Bot Token (get from [@BotFather](https://t.me/BotFather))
- LLM API Key (OpenAI or compatible service)

### Installation

```bash
git clone https://github.com/your-username/telegram-bot.git
cd telegram-bot
```

### Configuration

Copy the example environment file and modify it:

```bash
cp telegram-bot/.env.example .env
```

Edit the `.env` file:

```bash
# Telegram Bot Token (required)
BOT_TOKEN=your_bot_token_here

# OpenAI API Configuration (required)
OPENAI_API_KEY=your_api_key_here
OPENAI_BASE_URL=https://api.openai.com/v1  # or use compatible API
MODEL=gpt-3.5-turbo

# Streaming Response (optional)
USE_STREAMING=true
THINKING_MESSAGE=Thinking...

# Database Configuration
DATABASE_URL=file:./telegram_bot.db

# Memory Storage Configuration
MEMORY_STORE_TYPE=lance    # memory | sqlite | lance
MEMORY_RECENT_LIMIT=10     # number of recent messages
LANCE_DB_PATH=./data/lancedb

# Embedding Service Configuration
EMBEDDING_PROVIDER=openai  # openai | zhipuai
# BIGMODEL_API_KEY=your_key  # required when using Zhipu AI

# Log Level
RUST_LOG=info
```

### Running

```bash
cargo run --release -p telegram-llm-bot -- run
```

To use LanceDB as memory storage, add the `lance` feature: see [Enabling Lance (LanceDB)](#enabling-lance-lancedb).

## Memory System

The bot supports three context building strategies:

### RecentMessagesStrategy
Retrieves the most recent N conversation messages as context.

### SemanticSearchStrategy
Uses vector embeddings for semantic similarity search to find the most relevant historical conversations.

### UserPreferencesStrategy
Records and retrieves user preference settings.

## Storage Backends

| Type | Description | Use Case |
|------|-------------|----------|
| `memory` | In-memory storage | Development and testing |
| `sqlite` | SQLite storage | Lightweight deployment |
| `lance` | LanceDB vector database | Production, requires semantic search |

### Enabling Lance (LanceDB)

LanceDB is **optional** and is injected by `telegram-llm-bot` when the `lance` feature is enabled. The framework crate (`telegram-bot`) does not depend on Lance; the LLM CLI entry point (`telegram-llm-bot`) creates and injects the Lance store.

1. **Build or run with the `lance` feature:**

   ```bash
   # Run with Lance support (telegram-llm-bot only)
   cargo run --release -p telegram-llm-bot --features lance -- run
   ```

2. **Set memory store type to `lance`** in `.env`:

   ```bash
   MEMORY_STORE_TYPE=lance
   LANCE_DB_PATH=./data/lancedb
   ```

3. **Run tests:**

   ```bash
   cargo test -p memory-lance
   ```

The `memory-lance` crate is a workspace member and is built when running `cargo build --workspace`. Lance support in `telegram-llm-bot` requires `--features lance`.

## Documentation

- [Config 重构方案](docs/config-refactoring-plan.md)：可扩展配置架构的详细设计与迁移方案

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p memory

# Run integration tests
cargo test --test '*_integration_test'
```

### Code Structure

- **telegram-bot**: Core `Bot`, `Handler` traits, handler chain, Telegram adapter, and built-in handlers (logging, auth, memory, persistence)
- **telegram-llm-bot**: LLM integration (SyncLLMHandler, @mention detection and processing)
- **memory**: Memory management and context building
- **storage**: Message persistence

## Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| `BOT_TOKEN` | Telegram Bot Token | - |
| `OPENAI_API_KEY` | OpenAI API Key | - |
| `OPENAI_BASE_URL` | API Base URL | `https://api.openai.com/v1` |
| `MODEL` | Model to use | `gpt-3.5-turbo` |
| `USE_STREAMING` | Enable streaming responses | `false` |
| `THINKING_MESSAGE` | Thinking message | - |
| `DATABASE_URL` | Database connection | `file:./telegram_bot.db` |
| `MEMORY_STORE_TYPE` | Memory storage type | `memory` |
| `MEMORY_RECENT_LIMIT` | Number of recent messages | `10` |
| `MEMORY_RELEVANT_TOP_K` | Semantic search results | `5` |
| `LANCE_DB_PATH` | LanceDB path | `./data/lancedb` |
| `EMBEDDING_PROVIDER` | Embedding service provider | `openai` |
| `BIGMODEL_API_KEY` | Zhipu AI API Key | - |
| `RUST_LOG` | Log level | `info` |

## Using Zhipu AI (GLM)

This project supports Zhipu AI's GLM models:

```bash
OPENAI_BASE_URL=https://open.bigmodel.cn/api/paas/v4
OPENAI_API_KEY=your_zhipu_api_key
MODEL=glm-4-flash

# Use Zhipu embedding service
EMBEDDING_PROVIDER=zhipuai
BIGMODEL_API_KEY=your_zhipu_api_key
```

## License

MIT
