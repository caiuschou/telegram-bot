# dbot-core Crate Architecture

## Overview

`dbot-core` is the foundational crate for the Telegram bot framework. It provides core abstractions, data types, and utility functions that enable building modular, type-safe Telegram bots in Rust.

## Design Principles

- **Trait-based abstraction**: Core functionality is defined as traits to enable multiple implementations
- **Async-first**: All operations are async using `async-trait` for compatibility with tokio
- **Type safety**: Leverages Rust's type system with `serde` serialization support
- **Modularity**: Clean separation of concerns allows components to be developed independently

## Module Structure

```
dbot-core/src/
├── lib.rs      # Crate entry point, public API re-exports
├── bot.rs      # Bot trait and TelegramBot implementation
├── types.rs    # Core data types and handler/middleware traits
├── error.rs    # Error type definitions
└── logger.rs   # Custom tracing layer for file + console logging
```

## Core Components

### 1. Bot Abstraction (`bot.rs`)

The `Bot` trait defines the interface for sending messages:

```rust
#[async_trait]
pub trait Bot {
    async fn send_message(&self, chat: &Chat, text: &str) -> Result<()>;
    async fn reply_to(&self, message: &Message, text: &str) -> Result<()>;
}
```

**TelegramBot** - Concrete implementation using teloxide:
- Wraps `teloxide::Bot` internally
- Converts core types to teloxide types (e.g., `ChatId`)
- Maps teloxide errors to `DbotError::Bot`

### 2. Data Types (`types.rs`)

#### Core Domain Types

| Type | Description |
|------|-------------|
| `User` | Telegram user identity (id, username, names) |
| `Chat` | Chat information (id, chat_type) |
| `Message` | Complete message with content, direction, metadata |
| `MessageDirection` | Enum: `Incoming` or `Outgoing` |
| `HandlerResponse` | Enum: `Continue`, `Stop`, `Ignore` |

#### Conversion Traits

```rust
pub trait ToCoreUser: Send + Sync {
    fn to_core(&self) -> User;
}

pub trait ToCoreMessage: Send + Sync {
    fn to_core(&self) -> Message;
}
```

These traits allow external types (e.g., from teloxide) to be converted to core types.

#### Handler Trait

```rust
#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, message: &Message) -> crate::error::Result<HandlerResponse>;
}
```

Implemented by components that process messages. Return values control the handler chain:
- `Continue` - Pass to next handler
- `Stop` - Terminate processing
- `Ignore` - Skip this handler's result

#### Middleware Trait

```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn before(&self, message: &Message) -> crate::error::Result<bool>;
    async fn after(&self, message: &Message, response: &HandlerResponse) -> crate::error::Result<()>;
}
```

Pre/post-processing hooks for the message pipeline. The `before` hook returning `false` can prevent handler execution.

### 3. Error Handling (`error.rs`)

```rust
pub enum DbotError {
    Database(String),
    Bot(String),
    Handler(#[from] HandlerError),
    Config(String),
    Io(#[from] std::io::Error),
    Unknown(String),
}

pub enum HandlerError {
    NoText,
    InvalidCommand(String),
    Unauthorized,
    State(String),
    EmptyContent,
}

pub type Result<T> = std::result::Result<T, DbotError>;
```

Two-tier error hierarchy:
- `DbotError` - Top-level application errors
- `HandlerError` - Specific handler logic errors (auto-converted)

### 4. Logging (`logger.rs`)

**AppLayer** - Custom tracing subscriber layer:
- Dual output: console + file
- Thread-safe file operations via `Mutex`
- Custom timestamp formatting
- Structured logging support

```rust
pub fn init_tracing(log_file_path: &str) -> anyhow::Result<()>
```

## Message Processing Flow

```
┌─────────────────┐
│  Telegram API   │
└────────┬────────┘
         │ teloxide types
         ▼
┌─────────────────┐
│  ToCoreMessage  │  (conversion)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  core::Message  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Middleware.before│ ────► return false?
└────────┬────────┘         (skip handlers)
         │ true
         ▼
┌─────────────────┐
│  Handler.handle │ ────► HandlerResponse
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Middleware.after│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Bot Trait API  │  (send response)
└─────────────────┘
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `async-trait` | Async trait support |
| `chrono` | Timestamp handling |
| `serde` | Serialization/deserialization |
| `teloxide` | Telegram Bot API client |
| `thiserror` | Error derive macros |
| `anyhow` | Error propagation in logger |
| `tracing` | Structured logging framework |
| `tracing-subscriber` | Logging subscriber implementation |

## Integration with Other Crates

```
┌──────────────────────────────────────────────────────────┐
│                      telegram-bot                         │
│  (Bot implementation, teloxide integration)               │
└────────────────────────┬─────────────────────────────────┘
                         │ implements Bot
                         ▼
┌──────────────────────────────────────────────────────────┐
│                       dbot-core                           │
│  (Core types, traits, errors, logging)                    │
└────────────────────────┬─────────────────────────────────┘
                         │ provides
                         ▼
┌──────────────────────────────────────────────────────────┐
│                      bot-runtime                          │
│  (Handler/Middleware implementations, dispatcher)         │
└──────────────────────────────────────────────────────────┘
```

## Usage Example

```rust
use dbot_core::{Bot, TelegramBot, Message, Handler};

#[derive(Clone)]
struct MyHandler;

#[async_trait]
impl Handler for MyHandler {
    async fn handle(&self, message: &Message) -> dbot_core::Result<dbot_core::HandlerResponse> {
        Ok(dbot_core::HandlerResponse::Continue)
    }
}

// Create bot
let bot = TelegramBot::new(token.to_string());

// Process messages
let handler = MyHandler;
let response = handler.handle(&message).await?;
```

## Public API

The crate re-exports all public types via `lib.rs`:

```rust
pub use bot::{Bot, TelegramBot};
pub use error::{DbotError, HandlerError, Result};
pub use logger::init_tracing;
pub use types::{
    Chat, Handler, HandlerResponse, Message, MessageDirection,
    Middleware, ToCoreMessage, ToCoreUser, User,
};
```

## Future Considerations

- Additional bot implementations (e.g., mock for testing)
- Message queue integration
- Rate limiting support
- Extended middleware capabilities (request modification)
