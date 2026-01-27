# 未来扩展

## 1. 对话总结

**功能**：自动总结长对话，压缩记忆

**实现思路**：
- 检测长对话（超过N轮或超过M tokens）
- 调用OpenAI总结对话
- 用总结替换原始对话记录
- 保留原始对话在归档存储中

**优先级**：中
**预估工作量**：2-3天

```rust
pub struct ConversationSummarizer {
    max_turns: usize,
    max_tokens: usize,
}

impl ConversationSummarizer {
    pub async fn summarize_conversation(
        &self,
        user_id: i64,
        conversation_id: String,
    ) -> Result<String> {
        // 1. 获取对话历史
        let history = self.get_conversation_history(user_id, &conversation_id).await?;

        // 2. 判断是否需要总结
        if !self.need_summarize(&history) {
            return Ok(String::new());
        }

        // 3. 调用AI总结
        let summary = self.openai_client.summarize(&history).await?;

        // 4. 归档原始对话
        self.archive_conversation(&conversation_id).await?;

        // 5. 保存总结
        let summary_entry = MemoryEntry::new(
            user_id,
            0,
            MemoryRole::System,
            &summary,
            self.embedding_service.embed(&summary).await?,
        );
        self.memory_store.add_memory(&summary_entry).await?;

        Ok(summary)
    }
}
```

## 2. 时间衰减

**功能**：旧记忆重要性降低

**实现思路**：
- 为每条记忆添加时间戳
- 计算时间衰减因子（如：0.9^天数）
- 检索时应用衰减因子到相似度分数
- 过滤掉衰减过度的记忆

**优先级**：低
**预估工作量**：1-2天

```rust
pub fn decay_score(
    similarity: f32,
    days_old: i64,
    decay_rate: f32,
) -> f32 {
    let decay = decay_rate.powi(days_old);
    similarity * decay
}
```

## 3. 跨用户检索

**功能**：管理员可检索所有对话

**实现思路**：
- 添加权限检查
- 管理员可以按用户、时间范围、关键词检索
- 支持导出用户对话数据

**优先级**：低
**预估工作量**：2-3天

```rust
pub async fn admin_search_all(
    &self,
    query: &str,
    filters: SearchFilters,
) -> Result<Vec<MemoryEntry>> {
    self.check_admin_permission()?;

    let mut results = self.memory_store.search_all(query).await?;

    // 应用过滤条件
    if let Some(user_id) = filters.user_id {
        results.retain(|e| e.user_id == user_id);
    }

    if let Some(start_time) = filters.start_time {
        results.retain(|e| e.timestamp >= start_time);
    }

    if let Some(end_time) = filters.end_time {
        results.retain(|e| e.timestamp <= end_time);
    }

    Ok(results)
}
```

## 4. 记忆导出

**功能**：支持导出用户记忆数据

**实现思路**：
- 支持导出格式：JSON、CSV、Markdown
- 支持按时间范围导出
- 支持数据脱敏

**优先级**：中
**预估工作量**：1-2天

```rust
pub enum ExportFormat {
    Json,
    Csv,
    Markdown,
}

pub async fn export_memories(
    &self,
    user_id: i64,
    format: ExportFormat,
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> Result<String> {
    let memories = self.get_user_memories_filtered(user_id, start_time, end_time).await?;

    match format {
        ExportFormat::Json => serde_json::to_string_pretty(&memories).map_err(Into::into),
        ExportFormat::Csv => self.export_to_csv(&memories),
        ExportFormat::Markdown => self.export_to_markdown(&memories),
    }
}
```

## 5. 多模态记忆

**功能**：支持图片、文件记忆

**实现思路**：
- 支持图片的CLIP向量嵌入
- 支持文件的文本提取和嵌入
- 混合检索（文本+图片）

**优先级**：低
**预估工作量**：5-7天

```rust
pub enum MemoryContent {
    Text(String),
    Image(Vec<u8>),
    File { name: String, content: Vec<u8> },
}

pub struct MultiModalEmbeddingService {
    text_embedding: Arc<dyn EmbeddingService>,
    image_embedding: Arc<dyn ImageEmbeddingService>,
}

impl MultiModalEmbeddingService {
    pub async fn embed(&self, content: &MemoryContent) -> Result<Vec<f32>> {
        match content {
            MemoryContent::Text(text) => self.text_embedding.embed(text).await,
            MemoryContent::Image(data) => self.image_embedding.embed(data).await,
            MemoryContent::File { content, .. } => {
                let text = extract_text_from_file(content)?;
                self.text_embedding.embed(&text).await
            }
        }
    }
}
```

## 6. 情感分析

**功能**：记录对话情感，个性化回复

**实现思路**：
- 分析用户消息的情感（积极、消极、中性）
- 标记记忆条目的情感标签
- 根据情感调整AI回复风格

**优先级**：低
**预估工作量**：3-4天

```rust
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

pub struct SentimentAnalyzer {
    openai_client: OpenAIClient,
}

impl SentimentAnalyzer {
    pub async fn analyze(&self, text: &str) -> Result<Sentiment> {
        let response = self.openai_client.chat_completion(
            "gpt-3.5-turbo",
            vec![
                SystemMessage {
                    content: "分析以下文本的情感，只返回：positive/negative/neutral".to_string(),
                },
                UserMessage {
                    content: text.to_string(),
                },
            ],
        ).await?;

        match response.to_lowercase().as_str() {
            "positive" => Ok(Sentiment::Positive),
            "negative" => Ok(Sentiment::Negative),
            _ => Ok(Sentiment::Neutral),
        }
    }
}
```

## 7. 对话分段

**功能**：自动识别对话主题变化，分段存储

**实现思路**：
- 检测主题变化（语义相似度骤降）
- 为每个主题分段添加标识
- 提高检索准确性

**优先级**：中
**预估工作量**：2-3天

## 8. 智能遗忘

**功能**：根据重要性和使用频率选择性遗忘

**实现思路**：
- 计算记忆的重要性分数
- 跟踪记忆的使用频率
- 定期清理低重要性、低频率的记忆

**优先级**：低
**预估工作量**：2-3天

## 扩展优先级建议

**高优先级**：
- 对话总结
- 记忆导出

**中优先级**：
- 对话分段
- 多模态记忆（如果需求明确）

**低优先级**：
- 时间衰减
- 跨用户检索
- 情感分析
- 智能遗忘
