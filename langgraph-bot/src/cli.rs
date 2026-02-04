//! CLI for langgraph-bot binary.
//!
//! Parses subcommands and args; see `main.rs` for dispatch to `langgraph_bot::load` and
//! `langgraph_bot::checkpoint`.

use clap::{Parser, Subcommand};

/// Root CLI: holds a single subcommand. Parsed by `main.rs` and matched to load/checkpoint calls.
#[derive(Parser)]
#[command(name = "langgraph-bot")]
#[command(about = "Load or seed messages into langgraph short-term memory (SqliteSaver checkpoint)")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Subcommands; `Load`, `Seed`, `Chat`, `Info`, `Memory`, and (with feature) `Run` are handled in `main.rs`.
#[derive(Subcommand)]
pub enum Commands {
    /// Print short-term memory (checkpoint) summary: threads and message counts. Optionally limit to one thread.
    Memory {
        /// Path to Sqlite checkpoint database (same as load/seed/chat).
        #[arg(short, long, default_value = "checkpoint.db")]
        db: std::path::PathBuf,

        /// If set, print summary only for this thread; otherwise list all threads with summaries.
        #[arg(short, long)]
        thread_id: Option<String>,
    },

    /// Print loaded tools, LLM interface, embeddings, and memory info.
    Info {
        /// Path to Sqlite checkpoint database (same as seed/chat).
        #[arg(short, long, default_value = "checkpoint.db")]
        db: std::path::PathBuf,
    },

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

    /// Chat with persistent memory. Optional first message; then waits for input line by line. Exit with Ctrl+C.
    Chat {
        /// Optional first message. If omitted, only the interactive loop runs.
        #[arg(value_name = "MESSAGE")]
        message: Option<String>,

        /// Path to Sqlite checkpoint database (same as seed).
        #[arg(short, long, default_value = "checkpoint.db")]
        db: std::path::PathBuf,

        /// Stream LLM output token-by-token.
        #[arg(long, default_value = "true")]
        stream: bool,

        /// Enable debug logging (RUST_LOG=debug).
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run Telegram bot with ReAct agent. Reply to the bot or @mention to trigger. Requires --features telegram.
    #[cfg(feature = "telegram")]
    Run {
        /// Bot token. If omitted, BOT_TOKEN from env is used.
        #[arg(short, long)]
        token: Option<String>,

        /// Path to Sqlite checkpoint database (same as chat).
        #[arg(short, long, default_value = "checkpoint.db")]
        db: std::path::PathBuf,
    },
}
