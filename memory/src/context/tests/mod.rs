//! Unit tests for the context module.
//!
//! Tests live in `memory/src/context/tests/` and are grouped by component.
//!
//! ## Coverage
//!
//! | Component           | File                    | Covered API / behaviour |
//! |---------------------|-------------------------|---------------------------|
//! | `estimate_tokens`   | estimate_tokens_test.rs | Token estimation (empty, single char, words) |
//! | `Context`           | context_test.rs         | format_for_model (with/without system, preferences, recent vs semantic), to_messages, is_empty, exceeds_limit |
//! | `ContextMetadata`   | context_test.rs         | Construction and use in Context (via make_context) |
//! | `ContextBuilder`    | context_builder_test.rs | new, with_token_limit, for_user, for_conversation, with_query, with_strategy, with_system_message, build() aggregation |

mod estimate_tokens_test;
mod context_test;
mod context_builder_test;
