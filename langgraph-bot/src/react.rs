//! ReAct agent: Think → Act → Observe with persistent checkpointer.
//!
//! Builds a `StateGraph<ReActState>` with ThinkNode(ChatOpenAI), ActNode(MockToolSource),
//! ObserveNode; compiles with SqliteSaver so each invoke persists. Used by `run_chat` and
//! `create_react_runner`. See idea/langgraph-bot/react-chat-plan.md.

use anyhow::Result;
use langgraph::memory::{Checkpointer, JsonSerializer, RunnableConfig, SqliteSaver};
use langgraph::{
    ActNode, ChatOpenAI, CompiledStateGraph, Message, MockToolSource, ObserveNode, ReActState,
    StateGraph, ThinkNode, ToolSource, REACT_SYSTEM_PROMPT, END, START,
};
use std::path::Path;
use std::sync::Arc;

/// Runner that holds a checkpointer and compiled ReAct graph for chat. Created once per DB path.
///
/// **Interaction**: Built by `create_react_runner`; used by `run_chat` to load state, invoke, and return reply.
pub struct ReactRunner {
    checkpointer: Arc<dyn Checkpointer<ReActState>>,
    compiled: CompiledStateGraph<ReActState>,
}

fn make_config(thread_id: &str) -> RunnableConfig {
    RunnableConfig {
        thread_id: Some(thread_id.to_string()),
        checkpoint_id: None,
        checkpoint_ns: String::new(),
        user_id: None,
    }
}

/// Builds the ReAct graph (think → act → observe) with the given checkpointer.
/// Uses ChatOpenAI from env (OPENAI_MODEL, OPENAI_API_KEY) and MockToolSource::get_time_example().
///
/// **Validation**: Checks OPENAI_API_KEY exists; OPENAI_MODEL defaults to gpt-4o-mini.
async fn build_compiled_graph(
    checkpointer: Arc<dyn Checkpointer<ReActState>>,
) -> Result<CompiledStateGraph<ReActState>> {
    // Validate required environment variables
    std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set. Please set it in .env file or environment."))?;
    
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let tool_source = MockToolSource::get_time_example();
    let tools = tool_source
        .list_tools()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list tools from MockToolSource: {}", e))?;
    let llm = ChatOpenAI::new(model).with_tools(tools);
    let think = ThinkNode::new(Box::new(llm));
    let act = ActNode::new(Box::new(tool_source));
    let observe = ObserveNode::new();

    let mut graph = StateGraph::<ReActState>::new();
    graph
        .add_node("think", Arc::new(think))
        .add_node("act", Arc::new(act))
        .add_node("observe", Arc::new(observe))
        .add_edge(START, "think")
        .add_edge("think", "act")
        .add_edge("act", "observe")
        .add_edge("observe", END);

    let compiled = graph
        .compile_with_checkpointer(checkpointer.clone())
        .map_err(|e| anyhow::anyhow!("Failed to compile ReAct graph: {}", e))?;
    Ok(compiled)
}

/// Creates a ReactRunner for the given DB path: loads .env, creates SqliteSaver checkpointer,
/// builds and compiles the ReAct graph. Use the same runner for multiple chat turns.
pub async fn create_react_runner(db_path: impl AsRef<Path>) -> Result<ReactRunner> {
    dotenvy::dotenv().ok();
    let serializer = Arc::new(JsonSerializer);
    let checkpointer: Arc<dyn Checkpointer<ReActState>> = Arc::new(
        SqliteSaver::new(db_path.as_ref(), serializer)
            .map_err(|e| anyhow::anyhow!("SqliteSaver at {:?}: {}", db_path.as_ref(), e))?,
    );
    let compiled = build_compiled_graph(checkpointer.clone()).await?;
    Ok(ReactRunner {
        checkpointer,
        compiled,
    })
}

impl ReactRunner {
    /// Runs one chat turn: load persistent state for `thread_id`, append user message,
    /// ensure system prompt, invoke ReAct graph, persist, return last assistant reply.
    ///
    /// **Error context**: Includes thread_id and message length in error messages.
    pub async fn run_chat(&self, thread_id: &str, content: &str) -> Result<String> {
        let config = make_config(thread_id);
        let tuple = self
            .checkpointer
            .get_tuple(&config)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to get checkpoint for thread '{}': {}",
                    thread_id,
                    e
                )
            })?;
        let mut state: ReActState = tuple
            .map(|(cp, _)| cp.channel_values)
            .unwrap_or_default();

        state.messages.push(Message::User(content.to_string()));
        let has_system = state
            .messages
            .first()
            .map(|m| matches!(m, Message::System(_)))
            .unwrap_or(false);
        if !has_system {
            state
                .messages
                .insert(0, Message::system(REACT_SYSTEM_PROMPT));
        }

        let result = self
            .compiled
            .invoke(state, Some(config.clone()))
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to invoke ReAct graph for thread '{}' (message length: {}): {}",
                    thread_id,
                    content.len(),
                    e
                )
            })?;

        let reply = result
            .messages
            .iter()
            .rev()
            .find_map(|m| {
                if let Message::Assistant(s) = m {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        Ok(reply)
    }
}
