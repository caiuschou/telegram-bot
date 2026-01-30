//! dbot CLI: run Telegram bot, load messages to vector DB, list vectors. Config from env and optional CLI args.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use memory_lance::{LanceConfig, LanceVectorStore};
use memory_loader::{load, EmbeddingConfig, EmbeddingProvider, LoadConfig};
use telegram_bot::{BotConfig, run_bot};

#[derive(Parser)]
#[command(name = "dbot")]
#[command(about = "Telegram Bot CLI: run, load, list-vectors", long_about = None)]
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
    /// Load messages from SQLite to vector DB (Lance); embedding and DB URLs from env.
    Load {
        #[arg(short, long, default_value = "50")]
        batch_size: usize,
    },
    /// List recent N records from vector DB (Lance), ordered by time descending.
    ListVectors {
        #[arg(short, long, default_value = "100")]
        limit: usize,
        #[arg(long)]
        lance_db_path: Option<String>,
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
        Commands::ListVectors { limit, lance_db_path } => {
            handle_list_vectors(limit, lance_db_path).await
        }
    }
}

/// Loads embedding config from env: EMBEDDING_PROVIDER, EMBEDDING_MODEL, OPENAI_API_KEY, BIGMODEL_API_KEY.
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

/// Handle the load command.
///
/// Reads config from env, calls memory_loader::load to perform data load.
/// Initializes tracing so memory-loader info logs go to console.
async fn handle_load(batch_size: usize) -> Result<()> {
    // Init tracing so import info logs are visible
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

/// Handle list-vectors command.
///
/// Reads LANCE_DB_PATH, LANCE_EMBEDDING_DIM from env, connects to LanceDB,
/// calls LanceVectorStore::list_recent for latest N entries (desc by time) and prints.
fn lance_embedding_dim() -> usize {
    std::env::var("LANCE_EMBEDDING_DIM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1536)
}

async fn handle_list_vectors(
    limit: usize,
    lance_db_path: Option<String>,
) -> Result<()> {
    let db_path = lance_db_path
        .unwrap_or_else(|| {
            std::env::var("LANCE_DB_PATH").unwrap_or_else(|_| "./lancedb".to_string())
        });
    let embedding_dim = lance_embedding_dim();

    let config = LanceConfig {
        db_path: db_path.clone(),
        embedding_dim,
        ..LanceConfig::default()
    };

    let store = LanceVectorStore::with_config(config)
        .await
        .context("Connect to LanceDB (check LANCE_DB_PATH and LANCE_EMBEDDING_DIM)")?;

    let entries = store
        .list_recent(limit)
        .await
        .context("Query list_recent from LanceDB")?;

    if entries.is_empty() {
        println!("No records (path: {}).", db_path);
        return Ok(());
    }

    const CONTENT_PREVIEW_LEN: usize = 80;
    println!("Recent {} record(s) (path: {}):\n", entries.len(), db_path);
    println!(
        "{:<36} {:<26} {:<8} {:<12} {}",
        "id", "timestamp", "role", "user_id", "content_preview"
    );
    println!("{}", "-".repeat(120));

    for e in &entries {
        let preview = if e.content.len() <= CONTENT_PREVIEW_LEN {
            e.content.as_str()
        } else {
            e.content.get(..CONTENT_PREVIEW_LEN).unwrap_or(&e.content)
        };
        let preview = preview.replace('\n', " ");
        let user_id = e.metadata.user_id.as_deref().unwrap_or("-");
        println!(
            "{:<36} {:<26} {:<8} {:<12} {}",
            e.id,
            e.metadata.timestamp.format("%Y-%m-%d %H:%M:%S"),
            format!("{:?}", e.metadata.role),
            user_id,
            preview
        );
    }

    Ok(())
}
