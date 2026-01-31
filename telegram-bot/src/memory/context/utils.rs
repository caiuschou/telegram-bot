//! Utility functions for context building.

/// Estimates the token count for a text string.
pub fn estimate_tokens(text: &str) -> usize {
    ((text.len() as f64) / 4.0).ceil().max(1.0) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_coverage() {
        assert_eq!(estimate_tokens(""), 1);
        assert_eq!(estimate_tokens("x"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("Hello world"), 3);
    }
}
