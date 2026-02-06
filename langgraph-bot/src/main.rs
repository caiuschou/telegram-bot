//! Binary for langgraph-bot: load or seed messages into langgraph SqliteSaver checkpoint.
//!
//! Uses `langgraph_bot::load` and `langgraph_bot::checkpoint`; see `cli.rs` for CLI definition.

use anyhow::Result;
use clap::Parser;
use langgraph_bot::print_runtime_info;

mod chat;
mod cli;
mod commands;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::from_filename(".env").ok();
    let cli = Cli::parse();

    match cli.command {
        Commands::Info => print_runtime_info().await?,
        Commands::Memory => commands::print_memory_summary()?,
        Commands::Chat {
            message,
            stream,
            verbose,
        } => chat::run_chat_loop(message, stream, verbose).await?,
        Commands::Run { token } => langgraph_bot::run_telegram(token).await?,
        Commands::Load {
            messages,
            db,
            thread_id,
        } => commands::cmd_load(messages, &db, thread_id)?,
        Commands::Seed { db, thread_id } => commands::cmd_seed(&db, thread_id)?,
    }

    Ok(())
}
