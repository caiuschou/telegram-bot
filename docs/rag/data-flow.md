# 数据流

## 消息处理流程

```
1. 用户发送消息
   ↓
2. MemoryMiddleware.process()
   - 保存用户消息到记忆库
   - 检索相关历史对话
   - 构建上下文
   ↓
3. TelegramBotAI.get_ai_response_with_memory()
   - 接收上下文
   - 构建完整提示词
   - 调用OpenAI生成回复
   ↓
4. 保存AI回复到记忆库
   ↓
5. 返回回复给用户
```

## 上下文构建策略

```rust
pub async fn build_context(
    &self,
    user_id: i64,
    query: &str,
    memory_store: Arc<dyn MemoryStore>,
) -> Result<String> {
    let mut context_parts = Vec::new();

    // 1. 获取用户偏好（如果有的话）
    if self.include_relevant {
        let preferences = memory_store.get_preferences(user_id).await?;
        if !preferences.is_empty() {
            context_parts.push("用户偏好信息：".to_string());
            for pref in preferences.iter().take(3) {
                context_parts.push(format!("- {}", pref.content));
            }
        }
    }

    // 2. 获取最近对话（短期上下文）
    if self.include_recent {
        let recent = memory_store.get_recent_context(user_id, 5).await?;
        if !recent.is_empty() {
            context_parts.push("\n最近的对话：".to_string());
            for entry in recent {
                let role = if entry.role == MemoryRole::User { "用户" } else { "助手" };
                context_parts.push(format!("{}: {}", role, entry.content));
            }
        }
    }

    // 3. 检索相关历史（长期记忆）
    if self.include_relevant {
        let relevant = memory_store.search_relevant(user_id, query, 3).await?;
        if !relevant.is_empty() {
            context_parts.push("\n相关的历史对话：".to_string());
            for entry in relevant {
                let role = if entry.role == MemoryRole::User { "用户" } else { "助手" };
                context_parts.push(format!("{}: {}", role, entry.content));
            }
        }
    }

    Ok(context_parts.join("\n"))
}
```

## 数据流详细说明

### 1. 用户消息处理

**输入**：
- 用户发送的消息内容
- 用户ID
- 聊天ID

**处理步骤**：
1. 文本预处理（清理、分词）
2. 向量化（调用OpenAI Embedding API）
3. 创建MemoryEntry
4. 保存到记忆库

### 2. 上下文检索

**检索策略**：
- **最近上下文**：按时间倒序获取最近N条对话
- **相关历史**：基于语义相似度检索
- **用户偏好**：标记为偏好的记忆条目

**检索参数**：
- `recent_limit`: 最近对话数量（默认5）
- `relevant_top_k`: 相关历史数量（默认3）
- `max_context_tokens`: 最大上下文token数（默认2000）

### 3. AI生成

**提示词构建**：
```
[系统提示]
你是一个有用的助手，用中文回答问题。

[上下文]
以下是相关的历史对话上下文：
用户: 我喜欢喝咖啡
助手: 记住了，你喜欢喝咖啡。

[用户问题]
推荐一杯饮品
```

**处理步骤**：
1. 构建完整消息列表
2. 调用OpenAI Chat Completion API
3. 接收AI回复
4. 向量化AI回复
5. 保存到记忆库

### 4. 回复返回

**输出**：
- AI生成的回复内容
- 使用的上下文（可选，用于调试）
