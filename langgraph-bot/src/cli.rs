//! CLI for langgraph-bot binary.
//!
//! Parses subcommands and args; see `main.rs` for dispatch to `langgraph_bot::load` and
//! `langgraph_bot::checkpoint`.

use clap::{Parser, Subcommand};

/// Root CLI: holds a single subcommand. Parsed by `main.rs` and matched to load/checkpoint calls.
#[derive(Parser)]
#[command(name = "langgraph-bot")]
#[command(about = "ReAct agent CLI: chat, run Telegram bot, load/seed message preview. Short-term memory disabled.")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands; `Load`, `Seed`, `Chat`, `Info`, `Memory`, and (with feature) `Run` are handled in `main.rs`.
#[derive(Subcommand)]
pub enum Commands {
    /// Print memory status (short-term memory is disabled).
    Memory,

    /// Print loaded tools, LLM interface, embeddings, and memory info.
    Info,

    /// Load messages into the checkpoint for a thread. Without -m: use TELEGRAM_MESSAGES_DB (Telegram SQLite) if set (then -t is chat_id), else LANGGRAPH_MESSAGES_PATH (JSON).
    Load {
        /// Path to messages JSON. If omitted: TELEGRAM_MESSAGES_DB or LANGGRAPH_MESSAGES_PATH from .env.
        #[arg(short, long)]
        messages: Option<std::path::PathBuf>,

        /// Path to Sqlite checkpoint database (created if missing).
        #[arg(short, long, default_value = "checkpoint.db")]
        db: std::path::PathBuf,

        /// Thread ID for the conversation. If omitted, a new UUID is generated.
        #[arg(short, long)]
        thread_id: Option<String>,
    },

    /// Fill checkpoint with generated/seed messages (samples or synthetic from seed-messages).
    Seed {
        /// Path to Sqlite checkpoint database (created if missing).
        #[arg(short, long, default_value = "checkpoint.db")]
        db: std::path::PathBuf,

        /// Thread ID for the conversation. If omitted, a new UUID is generated.
        #[arg(short, long)]
        thread_id: Option<String>,
    },

    /// Chat with ReAct agent (no conversation history; each line is a fresh turn). Optional first message; then stdin line by line. Exit with Ctrl+C or /exit.
    Chat {
        /// Optional first message. If omitted, only the interactive loop runs.
        #[arg(value_name = "MESSAGE")]
        message: Option<String>,

        /// Stream LLM output token-by-token.
        #[arg(long, default_value = "true")]
        stream: bool,

        /// Enable debug logging (RUST_LOG=debug).
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run Telegram bot with ReAct agent. Reply to the bot or @mention to trigger.
    Run {
        /// Bot token. If omitted, BOT_TOKEN from env is used.
        #[arg(short, long)]
        token: Option<String>,
    },
}
