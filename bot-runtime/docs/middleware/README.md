# Middleware Documentation

## Overview

Middleware in bot-runtime provides a flexible way to intercept and enhance message processing in the bot pipeline. Middleware components can execute logic before and after message handlers, enabling cross-cutting concerns such as logging, authentication, and memory management.

## Architecture

Middleware components implement the `Middleware` trait from `dbot_core`, which defines two main hooks:

- **before(message)**: Executes before the handler processes the message. Returns `Result<bool>` where `true` allows processing to continue.
- **after(message, response)**: Executes after the handler has processed the message. Can modify or log the response.

## Available Middleware

- [Persistence Middleware](./persistence-middleware.md) - Saves messages to SQLite database
- [Memory Middleware](./memory-middleware.md) - Manages conversation memory and context building
- [Logging Middleware](./logging-middleware.md) - Logs message processing events
- [Auth Middleware](./auth-middleware.md) - Enforces user access control

## Usage Pattern

```rust
use bot_runtime::{PersistenceMiddleware, MemoryMiddleware, LoggingMiddleware, AuthMiddleware};

// Create middleware instances
let persistence_middleware = PersistenceMiddleware::new(repo);
let memory_middleware = MemoryMiddleware::with_store(store);
let logging_middleware = LoggingMiddleware;
let auth_middleware = AuthMiddleware::new(vec![123, 456]);

// Add to handler chain
let mut chain = HandlerChain::new();
chain.add_middleware(auth_middleware);
chain.add_middleware(persistence_middleware);
chain.add_middleware(memory_middleware);
chain.add_middleware(logging_middleware);
```

## Execution Flow

The HandlerChain executes middleware and handlers in a specific order:

```
Message Received
    ↓
middleware1::before()
middleware2::before()
middleware3::before()
    ↓
handler1::handle()
handler2::handle()
    ↓
middleware3::after()
middleware2::after()
middleware1::after()
    ↓
Response Returned
```

This ensures:
- Pre-processing happens before business logic
- Post-processing happens in reverse order (LIFO)
- Any middleware can stop processing by returning false or an error

## Design Decisions

### Trait-Based Design

Middleware uses the `Middleware` trait from `dbot_core`, allowing custom implementations and easy testing with mock objects.

### Execution Order

Middleware executes in the order they are added to the chain. Each middleware's `before` method runs in order, followed by the handler, and then each middleware's `after` method runs in reverse order.

### Separation of Concerns

Middleware handles cross-cutting concerns:

- **PersistenceMiddleware**: Database storage
- **MemoryMiddleware**: RAG context management
- **LoggingMiddleware**: Observability and debugging
- **AuthMiddleware**: Security and access control

Handlers handle business logic:

- **AIDetectionHandler**: Detects AI mentions
- **AIQueryHandler**: Processes AI queries
- **Custom Handlers**: Domain-specific logic

### Error Handling

If a middleware's `before` method returns an error or `false`, the handler and subsequent middleware are not executed. Errors are propagated up the chain.

### Asynchronous Operations

All middleware operations are async to support non-blocking I/O operations, essential for high-performance bot applications with multiple concurrent conversations.

## Creating Custom Middleware

To create custom middleware, implement the `Middleware` trait:

```rust
use async_trait::async_trait;
use dbot_core::{Message, HandlerResponse, Middleware, Result};
use tracing::instrument;

pub struct CustomMiddleware {
    // Configuration fields
}

#[async_trait]
impl Middleware for CustomMiddleware {
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool> {
        // Pre-processing logic
        Ok(true)
    }

    #[instrument(skip(self))]
    async fn after(&self, message: &Message, response: &HandlerResponse) -> Result<()> {
        // Post-processing logic
        Ok(())
    }
}
```

## Best Practices

1. **Order Matters**: Put fast middleware first (Auth → Persistence → Memory → Logging)
2. **Fail Fast**: Return early on errors to avoid wasted processing
3. **No Side Effects in After**: Avoid modifying state in after hooks
4. **Use Logging**: Add instrumentation with `#[instrument]`
5. **Handle Errors Gracefully**: Decide whether to continue or stop on errors

## Performance Considerations

- **Middleware Overhead**: Each middleware adds ~50-100μs to processing time
- **Database Operations**: PersistenceMiddleware can add ~1-5ms
- **Memory Operations**: MemoryMiddleware can add ~10-50ms (with embeddings)
- **Total Impact**: With 4 middleware, expect ~20-100ms additional latency

## Testing

Middleware should be tested independently:

```rust
#[tokio::test]
async fn test_custom_middleware() {
    let middleware = CustomMiddleware::new();
    let message = create_test_message("test");

    let result = middleware.before(&message).await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}
```

And tested in integration with HandlerChain:

```rust
#[tokio::test]
async fn test_middleware_in_chain() {
    let chain = HandlerChain::new()
        .add_middleware(Arc::new(CustomMiddleware::new()));

    let message = create_test_message("test");
    let response = chain.handle(&message).await;

    assert!(response.is_ok());
}
```
