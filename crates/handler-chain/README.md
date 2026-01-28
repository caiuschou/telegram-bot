# handler-chain

A flexible and extensible handler chain implementation for bot message processing.

## Overview

The `HandlerChain` provides a middleware and handler execution pattern for processing messages. It executes middleware in the "before" phase, then handlers, and finally middleware in the "after" phase (in reverse order).

## Features

- **Middleware Support**: Chain multiple middleware that can intercept and modify request processing
- **Handler Execution**: Execute multiple handlers in sequence
- **Lifecycle Hooks**: Middleware can run before and after handler execution
- **Control Flow**: Middleware can stop the entire chain by returning `false` in the `before` phase
- **Handler Responses**: Handlers can control flow with `Continue`, `Stop`, or `Ignore` responses

## Usage

```rust
use handler_chain::HandlerChain;
use dbot_core::{Handler, HandlerResponse, Middleware};
use std::sync::Arc;

// Create a new handler chain
let chain = HandlerChain::new()
    .add_middleware(Arc::new(MyMiddleware))
    .add_handler(Arc::new(MyHandler));

// Process a message
let response = chain.handle(&message).await?;
```

## Execution Flow

1. **Before Phase**: Execute all middleware `before()` methods in order
2. **Handler Phase**: Execute handlers in order until one returns `Stop`
3. **After Phase**: Execute all middleware `after()` methods in reverse order

## Middleware

Middleware can interrupt the chain execution:

```rust
#[async_trait::async_trait]
impl Middleware for MyMiddleware {
    async fn before(&self, message: &Message) -> Result<bool> {
        // Return false to stop the chain before handlers execute
        Ok(true)
    }

    async fn after(&self, message: &Message, response: &HandlerResponse) -> Result<()> {
        // Post-processing logic
        Ok(())
    }
}
```

## Handler Responses

- `Continue`: Continue to the next handler
- `Stop`: Stop the handler chain and prevent further handlers from executing
- `Ignore`: Ignore the handler and continue to the next one

## Dependencies

- `dbot-core`: Core types and traits
- `async-trait`: Async trait support
- `anyhow`: Error handling
- `tracing`: Structured logging
- `chrono`: Time handling (for tests)

## Testing

Run tests with:

```bash
cargo test -p handler-chain
```

## License

This package is part of the dbot project.
