# memory-strategies

Context building strategies for conversation memory. Used by the `memory` crate's `ContextBuilder` to assemble AI conversation context from a memory store.

## Strategies

- **RecentMessagesStrategy** – Retrieves the most recent messages by conversation or user.
- **SemanticSearchStrategy** – Embeds the user's query and performs vector similarity search for relevant messages.
- **UserPreferencesStrategy** – Extracts user preference phrases (e.g. "I like", "I prefer") from history.

## Dependencies

- **memory-core** – For `MemoryStore`, `MemoryEntry`, `MemoryRole`, and `StrategyResult`.
- **embedding** – For `EmbeddingService` (used by `SemanticSearchStrategy`).

## Usage

Typically used via the `memory` crate:

```rust
use memory::{ContextBuilder, RecentMessagesStrategy, SemanticSearchStrategy, UserPreferencesStrategy};
use memory_inmemory::InMemoryVectorStore;
use std::sync::Arc;

let store = Arc::new(InMemoryVectorStore::new());
let builder = ContextBuilder::new(store)
    .with_strategy(Box::new(RecentMessagesStrategy::new(20)))
    .with_token_limit(4096);
```

## Tests

Unit tests live in `src/strategies_test.rs` (separate from lib code per project convention).
