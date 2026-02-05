//! ReAct agent: runner built from langgraph CompiledStateGraph + checkpointer.
//!
//! Runner holds only `compiled` and `checkpointer`; each turn uses
//! `build_react_initial_state` + `compiled.stream` / `invoke`. See docs/langgraph-bot-update-plan.md.

use anyhow::{anyhow, Result};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use langgraph::{
    ActNode, CompiledStateGraph, Message, ObserveNode, ReActState, RunnableConfig, StateGraph,
    ThinkNode, END, REACT_SYSTEM_PROMPT, START,
};
use langgraph::memory::{CheckpointError, Checkpointer};
use langgraph::react_builder::{build_react_run_context, ReactBuildConfig};
use langgraph::stream::{StreamEvent, StreamMode};
use tokio_stream::StreamExt;

/// User profile for long-term identity: injected as a System message before each turn when provided.
#[derive(Debug, Clone)]
pub struct UserProfile {
    pub user_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

impl UserProfile {
    /// Formats as the System message string used in checkpoint.
    pub fn to_system_content(&self) -> String {
        let name = [
            self.first_name.as_deref().unwrap_or("").trim(),
            self.last_name.as_deref().unwrap_or("").trim(),
        ]
        .join(" ")
        .trim()
        .to_string();
        let name_display = if name.is_empty() {
            "-".to_string()
        } else {
            name
        };
        let username_display = self
            .username
            .as_deref()
            .map(|u| format!("@{}", u.trim()))
            .unwrap_or_else(|| "-".to_string());
        format!(
            "User profile: {} ({}), user_id: {}",
            name_display, username_display, self.user_id
        )
    }
}

/// ReAct runner: holds compiled graph and checkpointer only (see plan §2.1).
/// Fields are used by `run_chat_stream`.
pub struct ReactRunner {
    pub(super) compiled: CompiledStateGraph<ReActState>,
    pub(super) checkpointer: Arc<dyn langgraph::memory::Checkpointer<ReActState>>,
}

/// Builds initial ReActState for one turn: from checkpoint (if thread_id + checkpointer) or fresh with system + user.
///
/// Mirrors langgraph `build_react_initial_state` so we work with git dependency that may not export it.
async fn build_initial_state_for_turn(
    content: &str,
    checkpointer: &Arc<dyn Checkpointer<ReActState>>,
    config: &RunnableConfig,
    system_prompt: &str,
) -> Result<ReActState, CheckpointError> {
    let thread_id = config.thread_id.as_deref();
    if thread_id.is_some() {
        let tuple = checkpointer.get_tuple(config).await?;
        if let Some((checkpoint, _)) = tuple {
            let mut state = checkpoint.channel_values.clone();
            state.messages.push(Message::user(content.to_string()));
            state.tool_calls = vec![];
            state.tool_results = vec![];
            return Ok(state);
        }
    }
    Ok(ReActState {
        messages: vec![
            Message::system(system_prompt),
            Message::user(content.to_string()),
        ],
        tool_calls: vec![],
        tool_results: vec![],
        turn_count: 0,
    })
}

/// Returns the content of the last Assistant message in `state`, or empty string if none.
///
/// Used by `run_chat_stream` (plan §5.2) and by tests (plan stage 2).
/// Interacts with `ReActState::messages`; matches last `Message::Assistant(content)`.
pub fn last_assistant_content(state: &ReActState) -> String {
    state
        .messages
        .iter()
        .rev()
        .find_map(|m| match m {
            Message::Assistant(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Builds checkpointer, LLM, ToolSource, optional Store from env; compiles graph; returns runner with only compiled + checkpointer.
///
/// Requires `OPENAI_API_KEY` (and optionally `OPENAI_MODEL`, `OPENAI_BASE_URL`). Uses `ReactBuildConfig::from_env()` and `build_react_run_context` for checkpointer/tool_source/store; then builds the ReAct graph and compiles with checkpointer.
/// Builds a minimal ReactBuildConfig for the builder: db_path, thread_id, and MCP-related env.
/// OpenAI key/model are read in create_react_runner from env (git dependency may not expose them on config).
fn react_build_config_for_runner(db_path: &Path) -> ReactBuildConfig {
    let mcp_verbose = std::env::var("MCP_VERBOSE")
        .or_else(|_| std::env::var("VERBOSE"))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(false);
    ReactBuildConfig {
        db_path: Some(db_path.display().to_string()),
        thread_id: Some("_".to_string()),
        user_id: std::env::var("USER_ID").ok(),
        exa_api_key: std::env::var("EXA_API_KEY").ok(),
        mcp_exa_url: std::env::var("MCP_EXA_URL").unwrap_or_else(|_| "https://mcp.exa.ai/mcp".to_string()),
        mcp_remote_cmd: std::env::var("MCP_REMOTE_CMD").unwrap_or_else(|_| "npx".to_string()),
        mcp_remote_args: std::env::var("MCP_REMOTE_ARGS").unwrap_or_else(|_| "-y mcp-remote".to_string()),
        mcp_verbose,
    }
}

pub async fn create_react_runner(db_path: impl AsRef<Path>) -> Result<ReactRunner> {
    let db_path = db_path.as_ref();
    let config = react_build_config_for_runner(db_path);

    let ctx = build_react_run_context(&config)
        .await
        .map_err(|e| anyhow!("build_react_run_context: {}", e))?;

    let checkpointer = ctx
        .checkpointer
        .ok_or_else(|| anyhow!("checkpointer required (thread_id was set)"))?;

    let api_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|s: &String| !s.is_empty())
        .ok_or_else(|| anyhow!("OPENAI_API_KEY is required"))?;
    let model = std::env::var("OPENAI_MODEL")
        .ok()
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "gpt-4o-mini".to_string());
    let mut openai_config = async_openai::config::OpenAIConfig::new().with_api_key(api_key.as_str());
    if let Ok(base) = std::env::var("OPENAI_BASE_URL") {
        if !base.is_empty() {
            openai_config = openai_config.with_api_base(base.as_str());
        }
    }
    let llm = langgraph::ChatOpenAI::new_with_tool_source(
        openai_config,
        model.as_str(),
        ctx.tool_source.as_ref(),
    )
    .await
    .map_err(|e| anyhow!("ChatOpenAI::new_with_tool_source: {}", e))?;
    let llm: Box<dyn langgraph::LlmClient> = Box::new(llm);

    let think = ThinkNode::new(llm);
    let act = ActNode::new(ctx.tool_source);
    let observe = ObserveNode::with_loop();

    let mut graph = StateGraph::<ReActState>::new();
    graph
        .add_node("think", Arc::new(think))
        .add_node("act", Arc::new(act))
        .add_node("observe", Arc::new(observe))
        .add_edge(START, "think")
        .add_edge("think", "act")
        .add_edge("act", "observe")
        .add_edge("observe", END);

    let graph = if let Some(store) = &ctx.store {
        graph.with_store(store.clone())
    } else {
        graph
    };

    let compiled = graph
        .compile_with_checkpointer(Arc::clone(&checkpointer))
        .map_err(|e| anyhow!("compile_with_checkpointer: {}", e))?;

    Ok(ReactRunner {
        compiled,
        checkpointer,
    })
}

/// Prints nothing (stub).
pub async fn print_runtime_info(_db_path: impl AsRef<Path>) -> Result<()> {
    Ok(())
}

impl ReactRunner {
    /// Streams one turn: builds initial state, runs graph with Messages + Values modes,
    /// calls `on_chunk` for each token, returns last assistant content (plan §5.2).
    pub async fn run_chat_stream(
        &self,
        thread_id: &str,
        content: &str,
        mut on_chunk: impl FnMut(&str) + Send,
        user_profile: Option<&UserProfile>,
    ) -> Result<String> {
        let mut config = RunnableConfig::default();
        config.thread_id = Some(thread_id.to_string());

        let system_prompt = user_profile
            .map(|p| format!("{}\n\n{}", REACT_SYSTEM_PROMPT, p.to_system_content()))
            .unwrap_or_else(|| REACT_SYSTEM_PROMPT.to_string());

        let state = build_initial_state_for_turn(
            content,
            &self.checkpointer,
            &config,
            &system_prompt,
        )
        .await
        .map_err(|e| anyhow!("build_initial_state_for_turn: {}", e))?;

        let modes = HashSet::from([StreamMode::Messages, StreamMode::Values]);
        let mut stream = self.compiled.stream(state, Some(config), modes);

        let mut final_state: Option<ReActState> = None;
        while let Some(event) = stream.next().await {
            match &event {
                StreamEvent::Messages { chunk, .. } => on_chunk(&chunk.content),
                StreamEvent::Values(s) => final_state = Some(s.clone()),
                _ => {}
            }
        }

        let reply = final_state
            .as_ref()
            .map(last_assistant_content)
            .unwrap_or_default();
        Ok(reply)
    }
}
