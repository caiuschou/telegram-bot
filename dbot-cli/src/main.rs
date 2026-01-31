//! dbot CLI: run Telegram bot. Config from env and optional CLI args.

use anyhow::Result;
use clap::{Parser, Subcommand};
use telegram_bot::{BotConfig, run_bot};

#[derive(Parser)]
#[command(name = "dbot")]
#[command(about = "Telegram Bot CLI: run", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the Telegram bot (config from env; token can override BOT_TOKEN).
    Run {
        #[arg(short, long)]
        token: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { token } => {
            let config = BotConfig::load(token)?;
            run_bot(config).await
        }
    }
}
