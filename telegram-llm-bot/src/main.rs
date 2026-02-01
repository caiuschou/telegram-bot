//! telegram-llm-bot: Full integration and entry point. Combines telegram-bot CLI + LLM handler.

use anyhow::Result;
use clap::Parser;
use std::path::Path;
use telegram_llm_bot::run_bot_with_llm;
use telegram_bot::{load_config, Cli, Commands};

/// Load .env: workspace root first (override so .env wins over shell env), then cwd as fallback.
fn load_dotenv() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    if let Some(parent) = Path::new(manifest_dir).parent() {
        let env_path = parent.join(".env");
        if env_path.exists() {
            // Use override so .env values overwrite any shell env (e.g. empty SYSTEM_PROMPT)
            let _ = dotenvy::from_path_override(&env_path);
        }
    }
    dotenvy::dotenv().ok();
}

#[tokio::main]
async fn main() -> Result<()> {
    load_dotenv();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { token } => {
            let config = load_config(token)?;
            run_bot_with_llm(config).await
        }
    }
}
