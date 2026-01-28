# Persistence Middleware

## Overview

The `PersistenceMiddleware` is a middleware component that automatically persists incoming messages to a SQLite database. It provides persistent storage for all message interactions, enabling audit trails, message history, and data analytics.

## Purpose

Persistence middleware solves the following problems:

1. **Data Persistence**: Stores all messages in a durable database
2. **Audit Trail**: Maintains a complete record of all bot interactions
3. **Analytics**: Enables message statistics and usage analysis
4. **History Retrieval**: Provides historical message data for reporting

## Configuration

### PersistenceMiddleware

```rust
pub struct PersistenceMiddleware {
    repo: MessageRepository,
}
```

### Constructor

```rust
impl PersistenceMiddleware {
    pub fn new(repo: MessageRepository) -> Self {
        Self { repo }
    }
}
```

## Usage

### Basic Usage

```rust
use bot_runtime::PersistenceMiddleware;
use storage::MessageRepository;

// Create repository and middleware
let repo = MessageRepository::new("sqlite::memory:")
    .await
    .expect("Failed to create repository");
let middleware = PersistenceMiddleware::new(repo);

// Add to handler chain
let mut chain = HandlerChain::new();
chain.add_middleware(middleware);
```

### With Production Database

```rust
use bot_runtime::PersistenceMiddleware;
use storage::MessageRepository;

// Create with file-based SQLite database
let repo = MessageRepository::new("sqlite:./bot_messages.db")
    .await
    .expect("Failed to create repository");
let middleware = PersistenceMiddleware::new(repo);

// Add to handler chain
let mut chain = HandlerChain::new();
chain.add_middleware(middleware);
```

### Integration with Other Middleware

```rust
use bot_runtime::{HandlerChain, PersistenceMiddleware, MemoryMiddleware, AuthMiddleware};

let mut chain = HandlerChain::new();

// Auth middleware executes first
chain.add_middleware(AuthMiddleware::new(vec![123, 456]));

// Persistence middleware executes second
chain.add_middleware(PersistenceMiddleware::new(repo));

// Memory middleware executes third
chain.add_middleware(MemoryMiddleware::with_store(memory_store));

// Handlers execute last
chain.add_handler(handler);
```

## How It Works

### Message Processing Flow

```
1. User Message Received
   ↓
2. PersistenceMiddleware::before()
   - Extract message details
   - Create MessageRecord
   - Save to SQLite database
   - Return Ok(true) to continue
   ↓
3. Handler Processes Message
   - Business logic executes
   ↓
4. PersistenceMiddleware::after()
   - No-op (persistence is complete)
   ↓
5. Response Sent to User
```

### Message Record Structure

```rust
let record = MessageRecord {
    id: Uuid::new_v4().to_string(),
    user_id: message.user.id,
    chat_id: message.chat.id,
    username: message.user.username,
    first_name: message.user.first_name,
    last_name: message.user.last_name,
    message_type: message.message_type,
    content: message.content,
    direction: match message.direction {
        MessageDirection::Incoming => "received",
        MessageDirection::Outgoing => "sent",
    }.to_string(),
    created_at: Utc::now(),
};
```

### Database Schema

```sql
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    username TEXT,
    first_name TEXT,
    last_name TEXT,
    message_type TEXT NOT NULL,
    content TEXT NOT NULL,
    direction TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_messages_user_id ON messages(user_id);
CREATE INDEX idx_messages_chat_id ON messages(chat_id);
CREATE INDEX idx_messages_created_at ON messages(created_at);
CREATE INDEX idx_messages_direction ON messages(direction);
CREATE INDEX idx_messages_message_type ON messages(message_type);
```

## API Reference

### PersistenceMiddleware

```rust
pub struct PersistenceMiddleware {
    repo: MessageRepository,
}

impl PersistenceMiddleware {
    pub fn new(repo: MessageRepository) -> Self;
}
```

### Middleware Trait Implementation

```rust
#[async_trait]
impl Middleware for PersistenceMiddleware {
    /// Saves message to database before handler execution
    #[instrument(skip(self, message))]
    async fn before(&self, message: &Message) -> Result<bool>;

    /// No-op after handler execution
    #[instrument(skip(self))]
    async fn after(&self, _message: &Message, _response: &HandlerResponse) -> Result<()>;
}
```

## Implementation Details

### Before Hook

The `before` hook performs the following operations:

1. Extracts message information from the `Message` struct
2. Creates a `MessageRecord` with all relevant fields
3. Saves the record to the SQLite database using `MessageRepository`
4. Logs success or failure
5. Returns `Ok(true)` to allow handler execution

**Error Handling**: If saving fails, the error is logged and returned, stopping the handler chain.

### After Hook

The `after` hook is a no-op that always returns `Ok(())` because:

1. **Persistence Complete**: Message is already saved in the before hook
2. **No Post-Processing**: No cleanup needed after handler execution
3. **Minimal Overhead**: Fast execution with no operations

### Logging

The middleware uses `tracing` for instrumentation:

- `debug!`: Logs message details before saving
- `debug!`: Logs success after saving
- `error!`: Logs save failures

### Database Initialization

The `MessageRepository` automatically:

1. Creates the `messages` table if it doesn't exist
2. Creates indexes for common query patterns
3. Uses connection pooling for performance

## Design Decisions

### Why Save in Before Hook?

Saving in the `before` hook ensures:

1. **Consistency**: All messages are saved, even if handler fails
2. **Audit Trail**: Complete record of all received messages
3. **Error Handling**: Can stop processing if persistence fails
4. **Order**: Messages are saved before business logic executes

### Why SQLite?

Using SQLite provides:

1. **Simplicity**: No external database server required
2. **Portability**: Single file database
3. **Performance**: Fast for read/write operations
4. **Reliability**: ACID compliant
5. **Compatibility**: Works across platforms

### Why Save All Message Details?

Saving complete message data enables:

1. **Analytics**: Full message history for analysis
2. **Debugging**: Context for troubleshooting
3. **Reporting**: Detailed interaction reports
4. **Search**: Flexible querying capabilities

### Why Return Error on Failure?

Returning an error on save failure provides:

1. **Data Integrity**: Alert if messages aren't being saved
2. **Visibility**: Failures are logged and visible
3. **Control**: Can handle failures appropriately
4. **Security**: Prevent silent data loss

## Database Operations

### Querying Messages

```rust
use storage::MessageQuery;

// Get recent messages for a user
let query = MessageQuery {
    user_id: Some(123),
    chat_id: None,
    limit: Some(10),
};
let messages = repo.get_messages(&query).await?;

// Search by keyword
let messages = repo.search_messages("keyword", Some(50)).await?;

// Get message by ID
let message = repo.get_message_by_id("message-id").await?;

// Get recent messages by chat
let messages = repo.get_recent_messages_by_chat(456, 20).await?;
```

### Getting Statistics

```rust
let stats = repo.get_stats().await?;

println!("Total messages: {}", stats.total_messages);
println!("Sent messages: {}", stats.sent_messages);
println!("Received messages: {}", stats.received_messages);
println!("Unique users: {}", stats.unique_users);
println!("Unique chats: {}", stats.unique_chats);
```

### Cleanup

```rust
// Delete messages older than 30 days
let deleted = repo.cleanup_old_messages(30).await?;
println!("Deleted {} old messages", deleted);
```

## Performance Considerations

### Database Connection Pooling

The repository uses connection pooling:

```rust
pool_manager: SqlitePoolManager
```

Benefits:
- Reuses connections
- Reduces overhead
- Improves concurrency

### Indexing

Multiple indexes optimize queries:

- `user_id`: Fast user-specific queries
- `chat_id`: Fast chat-specific queries
- `created_at`: Fast time-based queries
- `direction`: Fast direction filtering
- `message_type`: Fast type filtering

### Batch Operations

For high-volume scenarios, consider:

1. **Async Processing**: Save messages asynchronously
2. **Batching**: Group multiple saves
3. **Caching**: Cache frequently accessed data
4. **Partitioning**: Partition old data

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_persistence_middleware_creation() {
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("Failed to create repository");
    let _middleware = PersistenceMiddleware::new(repo);
}

#[tokio::test]
async fn test_persistence_middleware_before() {
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("Failed to create repository");
    let middleware = PersistenceMiddleware::new(repo.clone());

    let message = create_test_message("Hello");
    let result = middleware.before(&message).await;

    assert!(result.is_ok());
    assert!(result.unwrap());
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_persistence_flow() {
    let repo = MessageRepository::new("sqlite::memory:")
        .await
        .expect("Failed to create repository");
    let middleware = PersistenceMiddleware::new(repo.clone());

    let message = create_test_message("Test message");
    middleware.before(&message).await.unwrap();

    let messages = repo.get_messages(&MessageQuery {
        user_id: Some(123),
        chat_id: None,
        limit: Some(10),
    }).await.unwrap();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Test message");
}
```

### Running Tests

```bash
cd bot-runtime
cargo test persistence_middleware
```

## Best Practices

### 1. Error Handling

Always handle persistence errors:

```rust
match middleware.before(&message).await {
    Ok(true) => {
    }
    Ok(false) => {
    }
    Err(e) => {
        error!(error = %e, "Failed to persist message");
        return Err(e);
    }
}
```

### 2. Database Location

Use a consistent database location:

```rust
// Development
let db_url = "sqlite:./dev_bot.db";

// Production
let db_url = "sqlite:/var/lib/bot/bot.db";

// Testing
let db_url = "sqlite::memory:";
```

### 3. Regular Cleanup

Periodically clean up old messages:

```rust
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_hours(24)).await;
        let deleted = repo.cleanup_old_messages(30).await;
        info!("Cleaned up {} old messages", deleted.unwrap_or(0));
    }
});
```

### 4. Monitoring

Monitor database size and performance:

```rust
let stats = repo.get_stats().await?;
info!("Database stats: {:?}", stats);

// Monitor database file size
let metadata = std::fs::metadata("bot.db")?;
info!("Database size: {} bytes", metadata.len());
```

## Related Documentation

- [Middleware Architecture](./README.md) - General middleware concepts
- [Memory Middleware](./memory-middleware.md) - RAG context management
- [MessageRepository](../../../storage/src/message_repo.rs) - Database operations
- [MessageQuery](../../../storage/src/models.rs) - Query types
- [Handler Chain](../README.md) - Chain architecture
