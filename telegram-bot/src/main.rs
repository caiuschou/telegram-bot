//! Binary for base Telegram bot (persistence + memory, no LLM).
//! Uses integrated CLI from dbot-cli.

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use telegram_bot::{load_config, run_bot, Cli, Commands, NoOpHandler};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { token } => {
            let config = load_config(token)?;
            run_bot(config, |_config, _components| Arc::new(NoOpHandler::new())).await
        }
    }
}
