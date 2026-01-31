//! dbot-llm-cli: Full integration and entry point. Combines telegram-bot CLI + dbot-llm.

use anyhow::Result;
use clap::Parser;
use dbot_llm::run_bot_with_llm;
use telegram_bot::{load_config, Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { token } => {
            let config = load_config(token)?;
            run_bot_with_llm(config).await
        }
    }
}
