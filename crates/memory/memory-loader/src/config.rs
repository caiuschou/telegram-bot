//! 加载与词向量配置
//!
//! 定义 EmbeddingProvider、EmbeddingConfig、LoadConfig、LoadResult，
//! 以及根据配置创建 EmbeddingService 的逻辑。
//! 外部依赖：OpenAI API / 智谱 API（由 provider 决定）。

use std::sync::Arc;

use bigmodel_embedding::BigModelEmbedding;
use embedding::EmbeddingService;
use openai_embedding::OpenAIEmbedding;

/// 词向量服务提供商
///
/// 与 .env 中 EMBEDDING_PROVIDER 对应：openai / zhipuai。
pub enum EmbeddingProvider {
    /// OpenAI 词向量
    OpenAI,
    /// 智谱 BigModel 词向量
    Zhipuai,
}

/// 词向量配置
///
/// 可从 .env 构造：EMBEDDING_PROVIDER、EMBEDDING_MODEL、OPENAI_API_KEY、BIGMODEL_API_KEY。
pub struct EmbeddingConfig {
    /// 词向量服务
    pub provider: EmbeddingProvider,
    /// 词向量模型名；None 表示使用该 provider 默认模型
    pub model: Option<String>,
    /// OpenAI API Key（provider=OpenAI 时必填）
    pub openai_api_key: String,
    /// 智谱 API Key（provider=Zhipuai 时必填）
    pub bigmodel_api_key: String,
}

/// 数据加载配置
///
/// 从环境变量读取的配置信息，用于连接 SQLite、LanceDB 和词向量服务。
pub struct LoadConfig {
    /// SQLite 数据库路径
    pub database_url: String,
    /// LanceDB 数据库路径
    pub lance_db_path: String,
    /// 词向量配置（OpenAI 或智谱）
    pub embedding: EmbeddingConfig,
    /// 批量处理大小
    pub batch_size: usize,
}

/// 加载结果
///
/// 记录数据加载的统计信息。
pub struct LoadResult {
    /// 消息总数
    pub total: usize,
    /// 成功加载的消息数
    pub loaded: usize,
    /// 耗时（秒）
    pub elapsed_secs: u64,
}

/// 返回当前 embedding 配置对应的向量维度
///
/// LanceDB 表 schema 的 embedding_dim 必须与此一致，否则写入会失败。
pub(crate) fn embedding_dim_for_config(config: &EmbeddingConfig) -> usize {
    match config.provider {
        EmbeddingProvider::OpenAI => config
            .model
            .as_deref()
            .map(|m| match m {
                "text-embedding-3-large" => 3072,
                "text-embedding-ada-002" => 1536,
                _ => 1536, // text-embedding-3-small 等默认
            })
            .unwrap_or(1536),
        EmbeddingProvider::Zhipuai => config
            .model
            .as_deref()
            .map(|m| if m.starts_with("embedding-3") { 2048 } else { 1024 })
            .unwrap_or(1024), // embedding-2 默认 1024
    }
}

/// 根据 EmbeddingConfig 创建 EmbeddingService
///
/// 外部依赖：OpenAI API 或智谱 API，由 config.provider 决定。
pub(crate) fn create_embedding_service(
    config: &EmbeddingConfig,
) -> Arc<dyn EmbeddingService + Send + Sync> {
    match config.provider {
        EmbeddingProvider::OpenAI => {
            let model = config
                .model
                .clone()
                .unwrap_or_else(|| "text-embedding-3-small".to_string());
            Arc::new(OpenAIEmbedding::new(config.openai_api_key.clone(), model))
        }
        EmbeddingProvider::Zhipuai => {
            let model = config
                .model
                .clone()
                .unwrap_or_else(|| "embedding-2".to_string());
            Arc::new(BigModelEmbedding::new(config.bigmodel_api_key.clone(), model))
        }
    }
}
