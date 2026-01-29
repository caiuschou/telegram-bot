# memory-core

Core types and traits for memory storage and context strategies. Used by the `memory` crate and `memory-strategies` crate.

## Contents

- **types** – `MemoryEntry`, `MemoryMetadata`, `MemoryRole`
- **store** – `MemoryStore` trait (add, get, update, delete, search_by_user, search_by_conversation, semantic_search)
- **strategy_result** – `StrategyResult` enum (Messages, Preferences, Empty) returned by context strategies

## Usage

Consumers typically use the `memory` crate, which re-exports `memory-core`. Use `memory-core` directly only if you need the core types/traits without the `memory` context and migration features.
