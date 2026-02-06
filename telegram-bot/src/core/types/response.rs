//! Handler chain result type.

/// Handler result for the chain. `Reply(text)` carries the response body so later handlers can use it in `after()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerResponse {
    /// Pass to next handler.
    Continue,
    /// Stop the chain; no response body.
    Stop,
    /// Skip this handler, try next.
    Ignore,
    /// Stop the chain and attach reply text (e.g. save AI response in a handler's `after()`).
    Reply(String),
}
