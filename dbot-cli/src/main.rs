use anyhow::Result;
use clap::Parser;
use telegram_bot::{BotConfig, run_bot};

#[derive(Parser)]
#[command(name = "dbot")]
#[command(about = "运行 Telegram Bot", long_about = None)]
#[command(version)]
struct Cli {
    /// Bot token（覆盖环境变量）
    #[arg(short, long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    let config = BotConfig::load(cli.token)?;
    run_bot(config).await
}
