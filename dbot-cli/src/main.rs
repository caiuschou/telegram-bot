use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use memory_loader::{load, EmbeddingConfig, EmbeddingProvider, LoadConfig};
use telegram_bot::{BotConfig, run_bot};

#[derive(Parser)]
#[command(name = "dbot")]
#[command(about = "Telegram Bot 工具", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 运行 Telegram Bot
    Run {
        /// Bot token（覆盖环境变量）
        #[arg(short, long)]
        token: Option<String>,
    },
    /// 加载消息到向量数据库
    Load {
        /// 批量处理大小
        #[arg(short, long, default_value = "50")]
        batch_size: usize,
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
        Commands::Load { batch_size } => {
            handle_load(batch_size).await
        }
    }
}

/// 从 .env 读取词向量配置
/// 
/// 环境变量：EMBEDDING_PROVIDER、EMBEDDING_MODEL、OPENAI_API_KEY、BIGMODEL_API_KEY。
fn load_embedding_config() -> Result<EmbeddingConfig> {
    let provider = match std::env::var("EMBEDDING_PROVIDER").as_deref() {
        Ok("zhipuai") => EmbeddingProvider::Zhipuai,
        _ => EmbeddingProvider::OpenAI,
    };
    let model = std::env::var("EMBEDDING_MODEL").ok();
    let openai_api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    let bigmodel_api_key = std::env::var("BIGMODEL_API_KEY").unwrap_or_default();

    match &provider {
        EmbeddingProvider::OpenAI if openai_api_key.is_empty() => {
            anyhow::bail!(
                "OPENAI_API_KEY is required when EMBEDDING_PROVIDER=openai (or unset). \
                 Set it in .env or environment."
            );
        }
        EmbeddingProvider::Zhipuai if bigmodel_api_key.is_empty() => {
            anyhow::bail!(
                "BIGMODEL_API_KEY is required when EMBEDDING_PROVIDER=zhipuai. \
                 Set it in .env or environment."
            );
        }
        _ => {}
    }

    Ok(EmbeddingConfig {
        provider,
        model,
        openai_api_key,
        bigmodel_api_key,
    })
}

/// 处理 load 命令
/// 
/// 从环境变量读取配置，调用 memory_loader::load 执行数据加载。
/// 会初始化 tracing，使 memory-loader 内部的 info 日志输出到控制台。
async fn handle_load(batch_size: usize) -> Result<()> {
    // 初始化 tracing，使导入过程的 info 日志可见
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
        )
        .with_target(false)
        .init();

    println!("Loading messages to vector database...");

    let embedding = load_embedding_config()
        .context("Load embedding config from .env (EMBEDDING_PROVIDER, OPENAI_API_KEY / BIGMODEL_API_KEY)")?;

    let config = LoadConfig {
        database_url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "file:./telegram_bot.db".to_string()),
        lance_db_path: std::env::var("LANCE_DB_PATH")
            .unwrap_or_else(|_| "./lancedb".to_string()),
        embedding,
        batch_size,
    };

    let result = load(config).await?;

    println!("Total: {}, Loaded: {}, Time: {}s",
        result.total, result.loaded, result.elapsed_secs);

    Ok(())
}
