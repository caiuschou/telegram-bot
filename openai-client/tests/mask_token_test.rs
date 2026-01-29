//! Unit tests for `mask_token`.
//! Verifies that API keys are masked for safe logging: first 7 + "***" + last 4 chars;
//! keys of length <= 11 are fully masked as "***".

use openai_client::mask_token;

#[test]
fn mask_token_short_returns_all_star() {
    assert_eq!(mask_token(""), "***");
    assert_eq!(mask_token("a"), "***");
    assert_eq!(mask_token("sk-12345"), "***");
    assert_eq!(mask_token("sk-proj-12"), "***");
}

#[test]
fn mask_token_long_shows_head_and_tail() {
    // Length > 11: show first 7 + "***" + last 4
    assert_eq!(mask_token("sk-proj-abcdefghijklmnop"), "sk-proj***mnop");
    // len 12: head 7 = "sk-proj", tail 4 = last 4 = "xyzw"
    assert_eq!(mask_token("sk-proj-xyzw"), "sk-proj***xyzw");
}

#[test]
fn mask_token_typical_openai_key() {
    // Typical OpenAI key is long; we expect first 7 and last 4 visible
    let key = "sk-proj-1234567890abcdefghijklmnopqrstuvwxyz";
    let masked = mask_token(key);
    assert!(masked.starts_with("sk-proj"));
    assert!(masked.ends_with("wxyz"));
    assert!(masked.contains("***"));
    assert_eq!(masked.len(), 7 + 3 + 4);
}
