//! Unit tests for [`openai_client::mask_token`].
//!
//! Ensures API keys are masked for safe logging: first 7 chars + `***` + last 4 chars.
//! Keys of length ≤ 11 are fully masked as `***` to avoid leaking any segment.

use openai_client::mask_token;

/// **Test: Short or empty tokens are fully masked.**
///
/// **Expected:** Any token of length ≤ 11 returns `"***"` (no prefix/suffix shown).
#[test]
fn mask_token_short_returns_all_star() {
    assert_eq!(mask_token(""), "***");
    assert_eq!(mask_token("a"), "***");
    assert_eq!(mask_token("sk-12345"), "***");
    assert_eq!(mask_token("sk-proj-12"), "***");
}

/// **Test: Long tokens show first 7 and last 4 characters.**
///
/// **Expected:** For length > 11, result is `head(7) + "***" + tail(4)`.
#[test]
fn mask_token_long_shows_head_and_tail() {
    assert_eq!(mask_token("sk-proj-abcdefghijklmnop"), "sk-proj***mnop");
    assert_eq!(mask_token("sk-proj-xyzw"), "sk-proj***xyzw");
}

/// **Test: Typical long OpenAI key format.**
///
/// **Expected:** Masked string starts with `sk-proj`, ends with last 4 chars, contains `***`, total length 14.
#[test]
fn mask_token_typical_openai_key() {
    let key = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
    let masked = mask_token(key);
    assert!(masked.starts_with("sk-proj"));
    assert!(masked.ends_with("wxyz"));
    assert!(masked.contains("***"));
    assert_eq!(masked.len(), 7 + 3 + 4);
}
