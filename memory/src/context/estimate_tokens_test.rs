//! Unit tests for `estimate_tokens` function.
//!
//! Tests the token estimation logic used for context size calculation.
//! Interacts with: context building (token limit checks), AI model APIs (context window).

use super::*;

#[test]
fn test_estimate_tokens() {
    assert_eq!(estimate_tokens("Hello"), 2);
    assert_eq!(estimate_tokens("Hello world"), 3);
    assert_eq!(estimate_tokens("a"), 1);
    assert_eq!(estimate_tokens(""), 1);
}
