# Handler Chain Package Extraction

## Overview

This document describes the extraction of `HandlerChain` from the `bot-runtime` package into a standalone `handler-chain` package.

## Motivation

The `HandlerChain` is a core component that provides middleware and handler execution patterns. By extracting it into a standalone package:

1. **Improved Modularity**: The handler chain logic is now isolated and can be reused across different bot implementations
2. **Better Separation of Concerns**: Runtime-specific code is separated from the core chain execution logic
3. **Easier Testing**: The chain can be tested independently from runtime dependencies
4. **Reusability**: Other packages can use the handler chain without depending on `bot-runtime`

## Changes Made

### New Package Created

**Location**: `crates/handler-chain/`

**Structure**:
```
crates/handler-chain/
├── Cargo.toml          # Package configuration
├── src/
│   └── lib.rs          # HandlerChain implementation and tests
└── README.md           # Package documentation
```

### Dependencies

**handler-chain/Cargo.toml**:
```toml
[dependencies]
dbot-core = { path = "../../dbot-core" }
async-trait = "0.1"
anyhow = "1.0"
tracing = "0.1"
chrono = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
```

### Files Modified

1. **Cargo.toml** (workspace root)
   - Added `"crates/handler-chain"` to workspace members

2. **bot-runtime/Cargo.toml**
   - Added `handler-chain = { path = "../crates/handler-chain" }` to dependencies
   - No changes to existing dependencies

3. **bot-runtime/src/lib.rs**
   - Removed `mod handler_chain;` declaration
   - Kept `pub use handler_chain::HandlerChain;` (re-export)

4. **bot-runtime/src/handler_chain.rs**
   - Deleted (moved to new package)

### Code Moved

The entire `HandlerChain` implementation was moved from `bot-runtime/src/handler_chain.rs` to `crates/handler-chain/src/lib.rs`:

- `HandlerChain` struct
- `HandlerChain::new()`
- `HandlerChain::add_middleware()`
- `HandlerChain::add_handler()`
- `HandlerChain::handle()`
- All unit tests (3 tests, all passing)

## Verification

### Build Verification

```bash
# Check handler-chain package builds
cargo check -p handler-chain

# Check bot-runtime package builds
cargo check -p bot-runtime

# Check entire project builds
cargo check
```

All checks passed successfully.

### Test Verification

```bash
# Run handler-chain package tests
cargo test -p handler-chain

# Run bot-runtime package tests
cargo test -p bot-runtime
```

All tests passed:
- `handler-chain`: 3 tests passed
- `bot-runtime`: 26 tests passed

## API Compatibility

The public API of `HandlerChain` remains unchanged:

```rust
// Usage in bot-runtime (unchanged)
use bot_runtime::HandlerChain;

let chain = HandlerChain::new()
    .add_middleware(middleware)
    .add_handler(handler);

let response = chain.handle(&message).await?;
```

The re-export in `bot-runtime/src/lib.rs` ensures backward compatibility for existing code.

## Future Considerations

### Potential Enhancements

1. **Builder Pattern**: Consider adding a builder pattern for more complex chain configuration
2. **Async Iterators**: Support for async iteration over handlers/middleware
3. **Error Recovery**: Middleware-level error recovery strategies
4. **Metrics**: Built-in metrics collection for chain execution

### Possible Extensions

1. **Parallel Execution**: Option to run handlers in parallel
2. **Conditional Middleware**: Middleware that only runs under certain conditions
3. **Chain Composition**: Ability to compose multiple chains together
4. **Middleware Context**: Shared context across middleware and handlers

## Migration Notes

For users of `bot-runtime`:
- No changes required; the API remains the same
- Import path remains `use bot_runtime::HandlerChain;`

For new projects:
- Can directly depend on `handler-chain` package
- Import path: `use handler_chain::HandlerChain;`

## Related Issues

N/A (This is a proactive refactoring)

## Date

January 28, 2026
