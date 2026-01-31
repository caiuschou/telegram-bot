//! CLI parser and config loading.

use anyhow::Result;
use clap::{Parser, Subcommand};
use telegram_bot::BotConfig;

#[derive(Parser)]
#[command(name = "dbot")]
#[command(about = "Telegram Bot CLI", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the Telegram bot (config from env; token can override BOT_TOKEN).
    Run {
        #[arg(short, long)]
        token: Option<String>,
    },
}

/// Load BotConfig from environment. If `token` is provided it overrides BOT_TOKEN.
pub fn load_config(token: Option<String>) -> Result<BotConfig> {
    BotConfig::load(token)
}
