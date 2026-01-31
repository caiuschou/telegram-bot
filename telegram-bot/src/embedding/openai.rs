//! OpenAI embedding service.

use async_trait::async_trait;
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use tracing::{debug, info, warn};

use super::EmbeddingService;

#[derive(Debug, Clone)]
pub struct OpenAIEmbedding {
    client: Client<async_openai::config::OpenAIConfig>,
    model: String,
}

impl OpenAIEmbedding {
    pub fn new(api_key: String, model: String) -> Self {
        let api_key = if api_key.is_empty() {
            std::env::var("OPENAI_API_KEY").unwrap_or_default()
        } else {
            api_key
        };
        let client = Client::with_config(
            async_openai::config::OpenAIConfig::new().with_api_key(api_key)
        );
        Self { client, model }
    }

    pub fn with_api_key(api_key: String) -> Self {
        Self::new(api_key, "text-embedding-3-small".to_string())
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[async_trait]
impl EmbeddingService for OpenAIEmbedding {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, anyhow::Error> {
        const EMBED_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        const LOG_PREVIEW_LEN: usize = 200;
        let text_preview = if text.len() <= LOG_PREVIEW_LEN {
            text.to_string()
        } else {
            format!("{}...", &text[..LOG_PREVIEW_LEN])
        };
        info!(model = %self.model, text_preview = %text_preview, text_len = text.len(), "step: embedding OpenAI embed request");

        let request = CreateEmbeddingRequestArgs::default()
            .model(self.model.clone())
            .input(vec![text])
            .build()?;
        let embeddings = self.client.embeddings();
        let create_future = embeddings.create(request);
        let response = match tokio::time::timeout(EMBED_TIMEOUT, create_future).await {
            Ok(Ok(r)) => { debug!("OpenAI embed response received"); r }
            Ok(Err(e)) => { warn!(error = %e, "OpenAI embed request failed"); return Err(e.into()); }
            Err(_) => {
                warn!(timeout_secs = EMBED_TIMEOUT.as_secs(), "OpenAI embed request timed out");
                return Err(anyhow::anyhow!("OpenAI embed request timed out after {} seconds", EMBED_TIMEOUT.as_secs()));
            }
        };
        let embedding = match response.data.first() {
            Some(item) => item.embedding.clone(),
            None => { warn!("OpenAI embed response has no embedding data"); return Err(anyhow::anyhow!("No embedding in response")); }
        };
        info!(dimension = embedding.len(), "step: embedding OpenAI embed done");
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        if texts.is_empty() {
            debug!("OpenAI embed_batch empty input, skipping");
            return Ok(vec![]);
        }
        info!(model = %self.model, batch_size = texts.len(), "step: embedding OpenAI embed_batch request");
        const EMBED_BATCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
        let inputs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let request = CreateEmbeddingRequestArgs::default()
            .model(self.model.clone())
            .input(inputs)
            .build()?;
        let embeddings = self.client.embeddings();
        let create_future = embeddings.create(request);
        let response = match tokio::time::timeout(EMBED_BATCH_TIMEOUT, create_future).await {
            Ok(Ok(r)) => { debug!("OpenAI embed_batch response received"); r }
            Ok(Err(e)) => { warn!(error = %e, "OpenAI embed_batch request failed"); return Err(e.into()); }
            Err(_) => {
                warn!(timeout_secs = EMBED_BATCH_TIMEOUT.as_secs(), "OpenAI embed_batch request timed out");
                return Err(anyhow::anyhow!("OpenAI embed_batch request timed out after {} seconds", EMBED_BATCH_TIMEOUT.as_secs()));
            }
        };
        let embeddings: Vec<Vec<f32>> = response.data.into_iter().map(|item| item.embedding).collect();
        if embeddings.len() != texts.len() {
            warn!(expected = texts.len(), got = embeddings.len(), "OpenAI embed_batch response count mismatch");
            return Err(anyhow::anyhow!("Expected {} embeddings, got {}", texts.len(), embeddings.len()));
        }
        let dimension = embeddings.first().map(|v| v.len()).unwrap_or(0);
        info!(count = embeddings.len(), dimension = dimension, "step: embedding OpenAI embed_batch done");
        Ok(embeddings)
    }
}
