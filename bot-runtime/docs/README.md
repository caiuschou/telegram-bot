# Bot Runtime Documentation

## Overview

Bot runtime provides the core message processing framework for the dbot project. It implements a middleware-based architecture that enables flexible, extensible, and testable bot behavior.

## Architecture

The bot runtime is built around several key components:

```
bot-runtime/
├── src/
│   ├── handler_chain.rs       # Chain-of-responsibility for handlers and middleware
│   ├── middleware.rs          # Basic middleware implementations
│   ├── persistence_middleware.rs  # Database persistence middleware
│   ├── memory_middleware.rs   # Memory management middleware
│   ├── ai_detection_handler.rs
│   ├── ai_query_handler.rs
│   └── state.rs               # State management
└── docs/
    └── middleware/            # Middleware documentation
        ├── README.md
        ├── persistence-middleware.md
        ├── memory-middleware.md
        ├── logging-middleware.md
        └── auth-middleware.md
```

## Core Concepts

### Handler Chain

A chain-of-responsibility pattern that processes messages through middleware and handlers:

```rust
use bot_runtime::{HandlerChain, PersistenceMiddleware, MemoryMiddleware, AIDetectionHandler};

let mut chain = HandlerChain::new();

// Add middleware (executes before handlers)
chain.add_middleware(PersistenceMiddleware::new(repo));
chain.add_middleware(MemoryMiddleware::with_store(store));

// Add handlers (executes in order)
chain.add_handler(AIDetectionHandler::new(bot_username, query_sender));

// Process message
chain.handle(message).await?;
```

### Middleware

Middleware intercepts message processing before and after handler execution:

```rust
#[async_trait]
pub trait Middleware {
    async fn before(&self, message: &Message) -> Result<bool>;
    async fn after(&self, message: &Message, response: &HandlerResponse) -> Result<()>;
}
```

Middleware executes in the order they are added. The `before` hooks run in order, followed by handlers, then `after` hooks run in reverse order.

### Handlers

Handlers implement business logic for processing messages:

```rust
#[async_trait]
pub trait Handler {
    async fn handle(&self, message: &Message) -> Result<HandlerResponse>;
}
```

## Architecture Changes

### Recent Refactoring (v0.2.0)

The bot runtime has been refactored to properly integrate middleware with the handler chain:

**Before:**
- `HandlerChain` only supported handlers
- `MessageHandler` was a handler that persisted messages
- Middleware trait existed but wasn't used

**After:**
- `HandlerChain` supports both middleware and handlers
- `PersistenceMiddleware` replaces `MessageHandler` for database persistence
- Middleware properly wraps handler execution with before/after hooks

**Benefits:**
- Clear separation of concerns (middleware for cross-cutting concerns, handlers for business logic)
- Flexible ordering of middleware components
- Proper execution flow: middleware::before → handlers → middleware::after
- Consistent architecture across all message processing

## Documentation

### Middleware

Detailed documentation for available middleware components:

- [Middleware Overview](./middleware/README.md) - General middleware concepts and usage
- [Persistence Middleware](./middleware/persistence-middleware.md) - Database message persistence
- [Memory Middleware](./middleware/memory-middleware.md) - Conversation memory management
- [Logging Middleware](./middleware/logging-middleware.md) - Message processing logging
- [Auth Middleware](./middleware/auth-middleware.md) - User access control

### Handlers

Documentation for handler components (coming soon):

- [AI Query Handler](./handlers/ai-query-handler.md) - AI-powered query processing
- [AI Detection Handler](./handlers/ai-detection-handler.md) - AI mention detection
- [Custom Handlers](./handlers/custom-handlers.md) - Creating custom handlers

### State Management

Documentation for state management (coming soon):

- [State Manager](./state/state-manager.md) - Application state management
- [State Types](./state/state-types.md) - Available state types

## Quick Start

### Basic Bot

```rust
use bot_runtime::{HandlerChain, AIQueryHandler};
use openai_client::OpenAIClient;

// Create AI client
let ai_client = OpenAIClient::new("api-key");

// Create handler chain
let mut chain = HandlerChain::new();
chain.add_handler(AIQueryHandler::new(ai_client));

// Handle message
chain.handle(message).await?;
```

### With Middleware

```rust
use bot_runtime::{HandlerChain, AIQueryHandler, MemoryMiddleware, LoggingMiddleware};
use memory_inmemory::InMemoryVectorStore;
use std::sync::Arc;

// Create components
let store = Arc::new(InMemoryVectorStore::new());
let ai_client = OpenAIClient::new("api-key");

// Create handler chain with middleware
let mut chain = HandlerChain::new();
chain.add_middleware(LoggingMiddleware);
chain.add_middleware(MemoryMiddleware::with_store(store));
chain.add_handler(AIQueryHandler::new(ai_client));

// Handle message
chain.handle(message).await?;
```

## Design Patterns

### Chain of Responsibility

Handlers and middleware form a processing chain where each component can:
- Process the message
- Pass it to the next component
- Stop processing early

### Middleware Pattern

Middleware provides cross-cutting concerns:
- Pre-processing: Authorization, logging, validation
- Post-processing: Response logging, metrics, cleanup

### Dependency Injection

Components are created externally and injected:
- Easy testing with mock dependencies
- Flexible configuration
- Separation of concerns

## Configuration

### Environment Variables

The bot runtime supports configuration through environment variables:

```bash
# AI Configuration
OPENAI_API_KEY=sk-...

# Memory Configuration
MEMORY_MAX_RECENT_MESSAGES=10
MEMORY_MAX_CONTEXT_TOKENS=4096

# Authorization
ALLOWED_USERS=123,456,789

# Logging
RUST_LOG=bot_runtime=debug
```

### Custom Configuration

For advanced configuration, use custom config structs:

```rust
use bot_runtime::{MemoryConfig, MemoryMiddleware};

let config = MemoryConfig {
    store: my_custom_store,
    max_recent_messages: 20,
    max_context_tokens: 8192,
    save_user_messages: true,
    save_ai_responses: false,
};

let middleware = MemoryMiddleware::new(config);
```

## Testing

### Unit Testing

```rust
#[tokio::test]
async fn test_handler_chain() {
    let mut chain = HandlerChain::new();
    chain.add_handler(MyTestHandler::new());

    let message = create_test_message("test");
    let result = chain.handle(message).await;

    assert!(result.is_ok());
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_full_pipeline() {
    let chain = create_test_chain();
    let message = create_test_message("hello");

    let response = chain.handle(message).await.unwrap();

    assert!(matches!(response, HandlerResponse::Continue));
}
```

## Performance

### Middleware Overhead

Middleware typically adds minimal overhead:
- **LoggingMiddleware**: ~50μs per message
- **AuthMiddleware**: ~100ns per message (Vec) / ~30ns (HashSet)
- **MemoryMiddleware**: ~1ms per message (with embedding)

### Handler Chain Performance

Chain processing is sequential but fast:
- **Empty chain**: <1μs
- **3 middleware + 1 handler**: ~1-5ms
- **With memory embedding**: ~10-50ms

### Optimization Tips

1. **Order Matters**: Put fast middleware first (Auth → Logging → Memory)
2. **Use HashSet**: For large allowlists in AuthMiddleware
3. **Async Boundaries**: Minimize async overhead in tight loops
4. **Caching**: Cache expensive operations where possible

## Error Handling

The bot runtime uses structured error handling:

```rust
pub type Result<T> = std::result::Result<T, DbotError>;

pub enum DbotError {
    Handler(HandlerError),
    Unknown(String),
}

pub enum HandlerError {
    Unauthorized,
    InvalidMessage,
    ProcessingError,
}
```

### Error Propagation

Errors propagate up the handler chain:
- Middleware `before` returning error stops processing
- Handler errors returned to caller
- Middleware `after` errors don't prevent response delivery

### Graceful Degradation

Some components degrade gracefully:
- MemoryMiddleware: Logs errors but continues processing
- AuthMiddleware: Blocks unauthorized access
- LoggingMiddleware: No impact on processing

## Debugging

### Enable Tracing

```bash
# Enable debug logging
RUST_LOG=bot_runtime=debug cargo run

# Enable tracing with spans
RUST_LOG=bot_runtime=trace,bot_runtime::middleware=debug
```

### Distributed Tracing

The bot runtime supports distributed tracing:

```rust
use tracing::{info, instrument};

#[instrument(skip(self))]
async fn handle(&self, message: &Message) -> Result<()> {
    info!("Processing message");
    // ...
}
```

### Structured Logging

All middleware uses structured logging:

```rust
info!(
    user_id = %message.user.id,
    username = %message.user.username.as_deref().unwrap_or("unknown"),
    "User authorized"
);
```

## Future Enhancements

Planned improvements to the bot runtime:

1. **Async Streaming**: Support streaming responses from AI
2. **Rate Limiting**: Built-in rate limiting middleware
3. **Metrics**: Prometheus metrics integration
4. **Circuit Breakers**: Fault tolerance for external services
5. **Context Propagation**: Better context management across handlers
6. **Parallel Processing**: Concurrent handler execution where safe

## Related Documentation

- [dbot-core](../../dbot-core/) - Core types and traits
- [memory](../../memory/) - Memory management crate
- [ai-integration](../../ai-integration/) - AI client integration
- [RAG Solution](../RAG_SOLUTION.md) - Overall RAG architecture
- [Telegram Bot Guide](../RUST_TELEGRAM_BOT_GUIDE.md) - Telegram integration guide

## Contributing

When contributing to the bot runtime:

1. **Add Tests**: All new features need unit tests
2. **Update Docs**: Document new components
3. **Follow Patterns**: Use existing patterns for consistency
4. **Performance**: Consider performance implications
5. **Error Handling**: Handle errors gracefully

See [AGENTS.md](../../AGENTS.md) for development guidelines.
