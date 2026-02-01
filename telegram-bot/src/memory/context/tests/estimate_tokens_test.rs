use crate::memory::context::*;

#[test]
fn test_estimate_tokens() {
    assert_eq!(estimate_tokens("Hello"), 2);
    assert_eq!(estimate_tokens("Hello world"), 3);
    assert_eq!(estimate_tokens("a"), 1);
    assert_eq!(estimate_tokens(""), 1);
}
