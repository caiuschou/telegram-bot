//! ReAct agent: Think → Act → Observe with persistent checkpointer.
//!
//! Builds a `StateGraph<ReActState>` with ThinkNode(ChatOpenAI), ActNode(MockToolSource),
//! ObserveNode; compiles with SqliteSaver so each invoke persists. Used by `run_chat` and
//! `create_react_runner`. See idea/langgraph-bot/react-chat-plan.md.
//!
//! **Long-term memory (user profile)**: Optional `UserProfile` can be passed into `run_chat` / `run_chat_stream`;
//! when set, a System message is injected after the ReAct system prompt so the model sees the current user's profile.

use anyhow::Result;
use async_openai::config::OpenAIConfig;
use langgraph::memory::{Checkpointer, JsonSerializer, RunnableConfig, SqliteSaver};
use langgraph::{
    ActNode, ChatOpenAI, CompiledStateGraph, McpToolSource, Message, MockToolSource, ObserveNode,
    ReActState, StateGraph, ThinkNode, ToolSource, ToolSpec, REACT_SYSTEM_PROMPT, END, START,
};
use langgraph::stream::{StreamEvent, StreamMode};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio_stream::StreamExt;

/// User profile for long-term identity: injected as a System message before each turn when provided.
///
/// **Interaction**: Caller (e.g. telegram-langgraph integration) fetches from MessageRepository or user table
/// and passes to `run_chat` / `run_chat_stream`. When `None`, no profile is injected.
#[derive(Debug, Clone)]
pub struct UserProfile {
    pub user_id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

impl UserProfile {
    /// Formats as the System message string used in checkpoint (same convention as memory-migration 3.3).
    pub fn to_system_content(&self) -> String {
        let name = [
            self.first_name.as_deref().unwrap_or("").trim(),
            self.last_name.as_deref().unwrap_or("").trim(),
        ]
        .join(" ")
        .trim()
        .to_string();
        let name_display = if name.is_empty() { "-".to_string() } else { name };
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

/// Returns true if `messages` already contains a System message whose content starts with "User profile:".
fn state_has_user_profile_system(messages: &[Message]) -> bool {
    messages.iter().any(|m| {
        if let Message::System(s) = m {
            s.starts_with("User profile:")
        } else {
            false
        }
    })
}

/// Inserts a System message built from `profile` after the first System message (ReAct prompt), or at index 0 if none.
fn inject_user_profile_into_state(state: &mut ReActState, profile: &UserProfile) {
    let content = profile.to_system_content();
    let insert_at = state
        .messages
        .iter()
        .position(|m| matches!(m, Message::System(_)))
        .map(|i| i + 1)
        .unwrap_or(0);
    state
        .messages
        .insert(insert_at, Message::System(content));
}

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

/// Builds the tool source: McpToolSource (Exa) when EXA_API_KEY is set, else MockToolSource (get_time).
/// Used by `build_compiled_graph` and `print_runtime_info`.
fn make_tool_source() -> Box<dyn ToolSource> {
    if let Ok(exa_key) = std::env::var("EXA_API_KEY") {
        let exa_url =
            std::env::var("MCP_EXA_URL").unwrap_or_else(|_| "https://mcp.exa.ai/mcp".to_string());
        let cmd = std::env::var("MCP_REMOTE_CMD").unwrap_or_else(|_| "npx".to_string());
        let args_str =
            std::env::var("MCP_REMOTE_ARGS").unwrap_or_else(|_| "-y mcp-remote".to_string());
        let mut args: Vec<String> = args_str.split_whitespace().map(String::from).collect();
        if !args.iter().any(|a| a.contains("mcp.exa.ai") || a == &exa_url) {
            args.push(exa_url);
        }
        match McpToolSource::new_with_env(
            cmd,
            args,
            vec![("EXA_API_KEY".to_string(), exa_key)],
            false,
        ) {
            Ok(mcp) => Box::new(mcp),
            Err(e) => {
                eprintln!("Warning: McpToolSource init failed ({}), using MockToolSource", e);
                Box::new(MockToolSource::get_time_example())
            }
        }
    } else {
        Box::new(MockToolSource::get_time_example())
    }
}

/// Builds the ReAct graph (think → act → observe) with the given checkpointer.
/// Uses ChatOpenAI from env (OPENAI_MODEL, OPENAI_API_KEY, OPENAI_BASE_URL or OPENAI_API_BASE) and MockToolSource::get_time_example().
///
/// **Validation**: Checks OPENAI_API_KEY exists; OPENAI_MODEL defaults to gpt-4o-mini.
async fn build_compiled_graph(
    checkpointer: Arc<dyn Checkpointer<ReActState>>,
) -> Result<CompiledStateGraph<ReActState>> {
    // Validate required environment variables
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY not set. Please set it in .env file or environment."))?;

    // OPENAI_BASE_URL is used by async-openai; OPENAI_API_BASE is fallback for compatibility
    let api_base = std::env::var("OPENAI_BASE_URL")
        .or_else(|_| std::env::var("OPENAI_API_BASE"))
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());

    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let openai_config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(api_base);

    let tool_source = make_tool_source();
    let tools = tool_source
        .list_tools()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list tools: {}", e))?;
    let llm = ChatOpenAI::with_config(openai_config, model).with_tools(tools);
    let think = ThinkNode::new(Box::new(llm));
    let act = ActNode::new(tool_source);
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

/// Prints loaded tools, LLM interface, embeddings, and memory info. Used by CLI `info` subcommand.
///
/// **Output**: Tools (name + description), LLM (model, api_base), Embeddings (none), Memory (SqliteSaver + path).
pub async fn print_runtime_info(db_path: impl AsRef<Path>) -> Result<()> {
    dotenvy::dotenv().ok();

    let tool_source = make_tool_source();
    let tools: Vec<ToolSpec> = tool_source
        .list_tools()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list tools: {}", e))?;

    let api_base = std::env::var("OPENAI_BASE_URL")
        .or_else(|_| std::env::var("OPENAI_API_BASE"))
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
    let tool_type = if std::env::var("EXA_API_KEY").is_ok() {
        "McpToolSource (Exa web search)"
    } else {
        "MockToolSource (get_time)"
    };

    println!("=== langgraph-bot Runtime Info ===\n");

    println!("## Tools ({} total, source: {})", tools.len(), tool_type);
    for (i, t) in tools.iter().enumerate() {
        let desc = t.description.as_deref().unwrap_or("(no description)");
        println!("  [{}] {} - {}", i + 1, t.name, desc);
    }

    println!("\n## LLM Interface");
    println!("  Model: {}", model);
    println!("  API Base: {}", api_base);

    println!("\n## Embeddings (词嵌入)");
    println!("  Not used (langgraph-bot does not use embeddings)");

    println!("\n## Memory (记忆)");
    println!("  Type: SqliteSaver (checkpointer)");
    println!("  Path: {}", db_path.as_ref().display());
    println!("  State: ReActState {{ messages, tool_calls, tool_results }}");

    Ok(())
}

impl ReactRunner {
    /// Runs one chat turn: load persistent state for `thread_id`, append user message,
    /// ensure system prompt, optionally inject user profile (long-term memory), invoke ReAct graph, persist, return last assistant reply.
    ///
    /// **User profile**: When `user_profile` is `Some`, a System message is inserted after the ReAct system prompt
    /// (unless one already exists) so the model sees the current user's identity. Caller typically fetches from DB.
    ///
    /// **Error context**: Includes thread_id and message length in error messages.
    pub async fn run_chat(
        &self,
        thread_id: &str,
        content: &str,
        user_profile: Option<&UserProfile>,
    ) -> Result<String> {
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
        if let Some(profile) = user_profile {
            if !state_has_user_profile_system(&state.messages) {
                inject_user_profile_into_state(&mut state, profile);
            }
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

    /// Runs one chat turn with streaming: prints LLM tokens as they arrive, returns final reply.
    ///
    /// Uses `compiled.stream()` with `StreamMode::Messages` and `StreamMode::Values`.
    /// `on_chunk` is called for each MessageChunk (e.g. to print tokens); the last Values
    /// state is used to extract the final Assistant reply.
    /// When `user_profile` is `Some`, injects a User profile System message (same as `run_chat`).
    pub async fn run_chat_stream(
        &self,
        thread_id: &str,
        content: &str,
        mut on_chunk: impl FnMut(&str) + Send,
        user_profile: Option<&UserProfile>,
    ) -> Result<String> {
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
        if let Some(profile) = user_profile {
            if !state_has_user_profile_system(&state.messages) {
                inject_user_profile_into_state(&mut state, profile);
            }
        }

        let modes: HashSet<StreamMode> =
            HashSet::from_iter([StreamMode::Messages, StreamMode::Values]);
        let mut stream = self.compiled.stream(state, Some(config.clone()), modes);

        let mut last_state: Option<ReActState> = None;
        while let Some(event) = stream.next().await {
            match event {
                StreamEvent::Messages { chunk, .. } => {
                    on_chunk(&chunk.content);
                }
                StreamEvent::Values(s) => {
                    last_state = Some(s);
                }
                _ => {}
            }
        }

        let reply = last_state
            .as_ref()
            .and_then(|s| {
                s.messages
                    .iter()
                    .rev()
                    .find_map(|m| {
                        if let Message::Assistant(text) = m {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
            })
            .unwrap_or_default();
        Ok(reply)
    }
}
