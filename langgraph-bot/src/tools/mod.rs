//! Tools and tool source composition for the ReAct agent.
//!
//! [`StoreGetToolSource`] wraps an upstream [`langgraph::ToolSource`] and adds the
//! `store_get` tool (semantic search over the injected store). When store and embedding
//! are set, `store_get` is listed and executed; otherwise only upstream tools are used.

mod store_get_tool_source;

pub use store_get_tool_source::{StoreGetToolSource, STORE_GET_TOOL_NAME};
