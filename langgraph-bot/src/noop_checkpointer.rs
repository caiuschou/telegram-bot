//! No-op checkpointer: does not persist state. Used when short-term memory is disabled.
//!
//! `get_tuple` always returns `None`; `put` is a no-op and returns a dummy id; `list` returns empty.

use langgraph::memory::{Checkpoint, CheckpointError, CheckpointListItem, Checkpointer};
use langgraph::ReActState;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Checkpointer that never persists: no history is loaded or saved.
/// Use with `create_react_runner` when short-term memory is disabled.
pub struct NoOpCheckpointer;

impl NoOpCheckpointer {
    pub fn new() -> Arc<dyn Checkpointer<ReActState>> {
        Arc::new(NoOpCheckpointer)
    }
}

impl Checkpointer<ReActState> for NoOpCheckpointer {
    fn put<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        _config: &'life1 langgraph::memory::RunnableConfig,
        _checkpoint: &'life2 Checkpoint<ReActState>,
    ) -> Pin<Box<dyn Future<Output = Result<String, CheckpointError>> + Send + 'async_trait>>
    where
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        Box::pin(async { Ok("noop".to_string()) })
    }

    fn get_tuple<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _config: &'life1 langgraph::memory::RunnableConfig,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Option<(Checkpoint<ReActState>, langgraph::memory::CheckpointMetadata)>,
                        CheckpointError,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        Box::pin(async { Ok(None) })
    }

    fn list<'life0, 'life1, 'life2, 'life3, 'async_trait>(
        &'life0 self,
        _config: &'life1 langgraph::memory::RunnableConfig,
        _limit: Option<usize>,
        _before: Option<&'life2 str>,
        _after: Option<&'life3 str>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<CheckpointListItem>, CheckpointError>> + Send + 'async_trait,
        >,
    >
    where
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
        'life3: 'async_trait,
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}
