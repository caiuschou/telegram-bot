# ContextBuilder Design Document

## Overview

The `ContextBuilder` is responsible for constructing the context for AI conversation by retrieving and organizing relevant information from the memory store.

## Goals

1. **Flexible Context Construction**: Support multiple context building strategies
2. **Token Management**: Efficiently manage context window size
3. **Semantic Relevance**: Retrieve semantically relevant historical context
4. **User Awareness**: Incorporate user-specific information and preferences

## Architecture

```
ContextBuilder
├── Strategies
│   ├── RecentMessagesStrategy
│   ├── SemanticSearchStrategy
│   └── UserPreferencesStrategy
├── TokenWindowManager
└── ContextFormatter
```

## Core Components

### 1. ContextBuilder

The main builder class that orchestrates context construction.

```rust
pub struct ContextBuilder<T: MemoryStore> {
    store: Arc<T>,
    strategies: Vec<Box<dyn ContextStrategy>>,
    token_limit: usize,
}

pub struct Context {
    pub system_message: Option<String>,
    pub conversation_history: Vec<String>,
    pub user_preferences: Option<String>,
    pub metadata: ContextMetadata,
}
```

### 2. Context Strategies

#### RecentMessagesStrategy
- Retrieves the most recent N messages from a conversation
- Configurable message count
- Respects conversation boundaries

#### SemanticSearchStrategy
- Performs semantic search based on current query
- Returns top K most relevant historical messages
- Filters by user and/or conversation

#### UserPreferencesStrategy
- Extracts and summarizes user preferences
- Based on conversation history and explicit preferences
- Returns formatted preferences context

### 3. TokenWindowManager

Manages the context window to stay within token limits.

```rust
pub struct TokenWindowManager {
    max_tokens: usize,
    current_tokens: usize,
}
```

**Features**:
- Calculate approximate token count for messages
- Truncate context if exceeds limit
- Prioritize recent or important messages

### 4. Context Types

```rust
pub enum ContextType {
    Recent,
    Semantic,
    UserPreferences,
    System,
}

pub struct ContextMetadata {
    pub user_id: Option<String>,
    pub conversation_id: Option<String>,
    pub total_tokens: usize,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
}
```

## Usage Flow

```rust
let builder = ContextBuilder::new(store)
    .with_strategy(RecentMessagesStrategy::new(10))
    .with_strategy(SemanticSearchStrategy::new(5))
    .with_strategy(UserPreferencesStrategy::new())
    .with_token_limit(4096);

let context = builder
    .for_user("user123")
    .for_conversation("conv456")
    .with_query("What should I do today?")
    .build()
    .await?;
```

## API Design

### Builder Pattern

```rust
impl<T: MemoryStore> ContextBuilder<T> {
    pub fn new(store: Arc<T>) -> Self { }

    pub fn with_strategy(mut self, strategy: Box<dyn ContextStrategy>) -> Self { }

    pub fn with_token_limit(mut self, limit: usize) -> Self { }

    pub fn for_user(mut self, user_id: &str) -> Self { }

    pub fn for_conversation(mut self, conversation_id: &str) -> Self { }

    pub fn with_query(mut self, query: &str) -> Self { }

    pub async fn build(&self) -> Result<Context, anyhow::Error> { }
}
```

### Context Formatting

```rust
impl Context {
    /// Format context for AI model input
    pub fn format_for_model(&self, model: ModelType) -> String { }

    /// Get formatted conversation history
    pub fn format_history(&self) -> Vec<String> { }

    /// Get system message with context
    pub fn format_system(&self) -> Option<String> { }
}
```

## Token Management Strategy

### Priority Order (when truncating):
1. System message (always included)
2. Recent messages (most recent first)
3. Semantically relevant messages (highest similarity first)
4. User preferences (if space permits)

### Token Estimation

```rust
pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: 1 token ≈ 4 characters (for English)
    // More sophisticated approach can use tiktoken
    (text.len() / 4).ceil() as usize
}
```

## Error Handling

```rust
pub enum ContextError {
    NoMemoryStore,
    TokenLimitExceeded { requested: usize, limit: usize },
    EmptyContext,
    StrategyError(Box<dyn Error>),
}
```

## Testing Strategy

1. **Unit Tests**:
   - Test individual strategies
   - Test token calculation
   - Test context formatting

2. **Integration Tests**:
   - Test full context building flow
   - Test with different memory stores
   - Test token limit handling

3. **Performance Tests**:
   - Measure context building time
   - Test with large conversation histories

## Future Enhancements

1. **Dynamic Strategy Selection**: Choose strategies based on query type
2. **Context Caching**: Cache frequently used contexts
3. **Adaptive Token Limits**: Adjust token limit based on model requirements
4. **Context Summarization**: Summarize older conversations to save tokens
5. **Multi-turn Context**: Maintain context across multiple turns

## File Structure

```
memory/src/
├── context.rs          # ContextBuilder and Context types
├── crates/memory-strategies/  # Context strategy implementations (independent crate)
└── token_manager.rs    # Token window management
```

## Dependencies

- Existing: `MemoryStore` trait, `MemoryEntry`, etc.
- External: None (uses only existing dependencies)
