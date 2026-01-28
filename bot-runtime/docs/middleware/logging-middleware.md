# Logging Middleware

## Overview

The `LoggingMiddleware` is a simple middleware component that provides comprehensive logging of message processing events in the bot runtime. It logs incoming messages, processing status, and responses for monitoring, debugging, and auditing purposes.

## Purpose

Logging middleware solves the following problems:

1. **Observability**: Provides visibility into message processing flow
2. **Debugging**: Enables troubleshooting of handler behavior
3. **Auditing**: Records all message interactions for compliance
4. **Performance**: Helps identify bottlenecks in message processing

## Usage

### Basic Usage

```rust
use bot_runtime::LoggingMiddleware;

// Create middleware
let middleware = LoggingMiddleware;

// Add to handler chain
let mut chain = HandlerChain::new();
chain.add_middleware(middleware);
```

### Integration with Other Middleware

```rust
use bot_runtime::{HandlerChain, LoggingMiddleware, MemoryMiddleware};

let mut chain = HandlerChain::new();

// Logging middleware executes first
chain.add_middleware(LoggingMiddleware);

// Memory middleware executes second
chain.add_middleware(MemoryMiddleware::with_store(store));

// Handlers execute last
chain.add_handler(handler);
```

## How It Works

### Message Processing Flow

```
1. User Message Received
   ↓
2. LoggingMiddleware::before()
   - Logs message details (INFO level):
     * User ID
     * Username
     * Message content
   - Returns Ok(true) to continue
   ↓
3. Handler Processes Message
   - Business logic executes
   ↓
4. LoggingMiddleware::after()
   - Logs processing result (DEBUG level):
     * Message ID
     * Response variant
   - Returns Ok(())
   ↓
5. Response Sent to User
```

## Logging Output

### Before Hook (INFO Level)

```log
INFO  Received message user_id=123 username="testuser" message_content="Hello, world!"
```

The log includes:
- **user_id**: Numeric user identifier
- **username**: User's username (or "unknown" if not set)
- **message_content**: Full message text

### After Hook (DEBUG Level)

```log
DEBUG Processed message message_id="msg_abc123" response=Continue
```

The log includes:
- **message_id**: Unique message identifier
- **response**: HandlerResponse variant (Continue, Stop, Ignore)

## API Reference

### LoggingMiddleware

```rust
pub struct LoggingMiddleware;
```

Zero-sized struct with no fields or constructor methods.

### Middleware Trait Implementation

```rust
#[async_trait]
impl Middleware for LoggingMiddleware {
    /// Logs incoming message details
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool>;

    /// Logs processing result
    #[instrument(skip(self, message, response))]
    async fn after(&self, message: &Message, response: &HandlerResponse) -> Result<()>;
}
```

## Implementation Details

### Before Hook

The `before` hook performs the following operations:

1. Extracts user information from `message.user`
2. Extracts message content from `message.content`
3. Logs at INFO level with structured fields
4. Returns `Ok(true)` to allow handler execution

**Field Handling**:
- `username`: Uses `username.as_deref().unwrap_or("unknown")` to handle missing usernames
- `message_content`: Direct string formatting with `%` for display

### After Hook

The `after` hook performs the following operations:

1. Extracts message ID from `message.id`
2. Extracts response variant from `response`
3. Logs at DEBUG level with structured fields
4. Returns `Ok(())` for successful execution

**Field Handling**:
- `message_id`: Uses `?` debug formatting for clarity
- `response`: Uses `?` debug formatting to show variant

### Instrumentation

Both methods use `#[instrument]` attribute:

- Automatically creates a span for each call
- Skips large parameters with `skip()` to avoid bloat
- Enables distributed tracing and performance analysis

### Logging Levels

**INFO Level (before)**:
- Used for incoming messages
- Suitable for production monitoring
- Indicates significant events

**DEBUG Level (after)**:
- Used for processing results
- Suitable for development/debugging
- Can be disabled in production

## Design Decisions

### Why Two Levels?

Using INFO for `before` and DEBUG for `after` provides:

1. **Production Monitoring**: See all incoming messages without noise
2. **Debug Control**: Disable response logging in production
3. **Cost Efficiency**: Reduce log volume by filtering DEBUG logs
4. **Flexibility**: Adjust levels per environment

### Why Structured Logging?

Using structured fields with `tracing` provides:

1. **Queryability**: Filter logs by user_id, username, etc.
2. **Consistency**: Standardized log format across components
3. **Integration**: Compatible with log aggregation tools
4. **Performance**: Efficient structured logging

### Why No Configuration?

The middleware has no configuration options because:

1. **Simplicity**: Works out of the box with sensible defaults
2. **Predictability**: Always logs the same information
3. **Flexibility**: Use `tracing` filters to adjust logging behavior
4. **Low Overhead**: Minimal runtime configuration needed

### Why Unit Struct?

Using a unit struct (`LoggingMiddleware`) instead of a struct with fields provides:

1. **Zero Cost**: No memory allocation
2. **Singleton Pattern**: One instance serves all uses
3. **Type Safety**: Can't be confused with other middleware
4. **Copy/Send/Sync**: Automatically implements these traits

## Testing

The module does not include explicit tests because:

1. **Trivial Implementation**: Only logging, no complex logic
2. **External Dependency**: Testing requires tracing subscriber
3. **Coverage**: Covered by integration tests of handler chain
4. **Best Practice**: Don't test external libraries (tracing)

### Manual Testing

```rust
// Initialize tracing subscriber
tracing_subscriber::fmt::init();

// Create and use middleware
let middleware = LoggingMiddleware;
middleware.before(&message).await.unwrap();
```

## Configuration Examples

### Enable DEBUG Logs

```rust
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(
        EnvFilter::from_default_env()
            .add_directive("bot_runtime::middleware=debug".parse().unwrap())
    )
    .init();
```

### Customize Log Format

```rust
tracing_subscriber::fmt()
    .with_target(false) // Don't show module path
    .with_thread_ids(true) // Show thread IDs
    .with_level(true) // Show log level
    .init();
```

### Log to File

```rust
use tracing_appender::rolling;

let file_appender = rolling::daily("./logs", "bot.log");
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

tracing_subscriber::fmt()
    .with_writer(non_blocking)
    .init();
```

## Related Documentation

- [Middleware Architecture](./README.md) - General middleware concepts
- [Tracing Documentation](https://docs.rs/tracing/) - Official tracing crate docs
- [Logging Guide](../../LOGGING.md) - Project logging guidelines
- [Memory Middleware](./memory-middleware.md) - Example of stateful middleware
