# OpenAI Client

一个简单的 OpenAI API Rust 客户端，支持流式和非流式响应。

## 功能

- ChatGPT API 调用
- 流式响应支持
- 自定义 API 地址
- 每 5 秒更新一次流式响应

## 使用方法

### 环境变量配置

创建 `.env` 文件并配置以下变量：

```env
OPENAI_API_KEY=your_openai_api_key_here
OPENAI_BASE_URL=https://api.openai.com/v1
```

### 非流式响应

```rust
use openai_client::OpenAIClient;

let client = OpenAIClient::new(api_key.to_string());
let messages = vec![
    // 创建消息...
];
let response = client.chat_completion("gpt-3.5-turbo", messages).await?;
```

### 流式响应

```rust
use openai_client::OpenAIClient;

let client = OpenAIClient::with_base_url(api_key.to_string(), base_url.to_string());

let messages = vec![
    // 创建消息...
];

client.chat_completion_stream("gpt-3.5-turbo", messages, |chunk| {
    println!("Chunk: {}", chunk.content);
    Ok(())
}).await?;
```

## 配置说明

- `OPENAI_API_KEY`: OpenAI API 密钥
- `OPENAI_BASE_URL`: OpenAI API 地址（可选，默认为 https://api.openai.com/v1）
