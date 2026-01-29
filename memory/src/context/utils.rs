//! Utility functions for context building.
//!
//! Token estimation and logging helpers. External: AI model token limits, cost calculation.

/// Estimates the token count for a text string.
///
/// This is a rough approximation: 1 token ≈ 4 characters for English text.
/// For production use, consider using tiktoken for more accurate estimation.
///
/// # Algorithm
///
/// Divides text length by 4 and rounds up, ensuring minimum of 1 token.
///
/// # External Interactions
///
/// - **AI Models**: Token count determines if context fits within model's context window
/// - **Cost Calculation**: Token usage directly impacts API costs for LLM providers
pub fn estimate_tokens(text: &str) -> usize {
    ((text.len() as f64) / 4.0).ceil().max(1.0) as usize
}

