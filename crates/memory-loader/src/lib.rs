//! # Memory Loader
//!
//! Load messages from SQLite to LanceDB vector store.
//!
//! See README.md for design and development plan.

mod config;
mod converter;

#[cfg(test)]
mod converter_test;

pub use config::{EmbeddingConfig, EmbeddingProvider, LoadConfig, LoadResult};

use anyhow::Result;
use memory_core::MemoryStore;
use memory_lance::{LanceConfig, LanceVectorStore};
use storage::{MessageQuery, MessageRepository};
use tracing::info;

use config::{create_embedding_service, embedding_dim_for_config};
use converter::convert;

/// 执行数据加载
///
/// 从 SQLite 读取消息，生成 embedding，写入 LanceDB。
///
/// # 流程
///
/// 1. 连接 SQLite (MessageRepository)
/// 2. 连接 LanceDB (LanceVectorStore)
/// 3. 初始化 EmbeddingService（按 config.embedding.provider 选择 OpenAI 或智谱）
/// 4. 获取消息总数
/// 5. 批量循环：读取 → 转换 → embedding → 写入
/// 6. 返回 LoadResult
///
/// # 参数
///
/// * `config` - 加载配置
///
/// # 返回
///
/// 返回 LoadResult 或错误
pub async fn load(config: LoadConfig) -> Result<LoadResult> {
    let start_time = std::time::Instant::now();

    info!("Starting data load process");

    // 1. 连接 SQLite
    info!("Connecting to SQLite: {}", config.database_url);
    let msg_repo = MessageRepository::new(&config.database_url).await?;

    // 2. 连接 LanceDB（向量维度需与当前 embedding 服务一致）
    let embedding_dim = embedding_dim_for_config(&config.embedding);
    info!(
        "Connecting to LanceDB: {} (embedding_dim={})",
        config.lance_db_path, embedding_dim
    );
    let lance_config = LanceConfig {
        db_path: config.lance_db_path.clone(),
        embedding_dim,
        ..LanceConfig::default()
    };
    let vector_store = LanceVectorStore::with_config(lance_config).await?;

    // 3. 初始化 EmbeddingService（按 provider 选择 OpenAI 或智谱）
    let provider_name = match config.embedding.provider {
        EmbeddingProvider::OpenAI => "OpenAI",
        EmbeddingProvider::Zhipuai => "Zhipuai",
    };
    info!("Initializing embedding service: {}", provider_name);
    let embedding_service = create_embedding_service(&config.embedding);

    // 4. 获取消息总数
    let stats = msg_repo.get_stats().await?;
    let total = stats.total_messages as usize;
    info!("Total messages to load: {}", total);

    if total == 0 {
        info!("No messages to load");
        return Ok(LoadResult {
            total: 0,
            loaded: 0,
            elapsed_secs: 0,
        });
    }

    // 5. 批量循环处理（按 offset 分页）
    let mut loaded = 0;
    let mut offset: i64 = 0;

    loop {
        info!(
            "Loading batch at offset {}, batch_size {}",
            offset, config.batch_size
        );
        let query = MessageQuery {
            user_id: None,
            chat_id: None,
            message_type: None,
            direction: None,
            start_date: None,
            end_date: None,
            limit: Some(config.batch_size as i64),
            offset: Some(offset),
        };

        let messages = msg_repo.get_messages(&query).await?;
        if messages.is_empty() {
            break;
        }

        let mut entries: Vec<_> = messages.iter().map(convert).collect();

        let texts: Vec<String> = entries.iter().map(|e| e.content.clone()).collect();
        info!("Generating embeddings for {} messages", texts.len());
        let embeddings: Vec<Vec<f32>> = embedding_service.embed_batch(&texts).await?;

        for (entry, embedding) in entries.iter_mut().zip(embeddings.iter()) {
            entry.embedding = Some(embedding.clone());
        }

        info!("Writing {} entries to LanceDB", entries.len());
        for entry in entries {
            vector_store.add(entry).await?;
            loaded += 1;
        }

        info!("Progress: {}/{} messages loaded", loaded, total);

        offset += messages.len() as i64;

        if messages.len() < config.batch_size {
            break;
        }
    }

    let elapsed_secs = start_time.elapsed().as_secs();
    info!(
        "Data load completed: total={}, loaded={}, elapsed={}s",
        total, loaded, elapsed_secs
    );

    Ok(LoadResult {
        total,
        loaded,
        elapsed_secs,
    })
}
