//! dbot-llm-cli: Full integration and entry point. Combines dbot-cli base + dbot-llm.

use anyhow::Result;
use clap::Parser;
use dbot_cli::{load_config, Cli, Commands};
use dbot_llm::run_bot_with_llm;

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
