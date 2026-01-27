# Telegram Bot AI

这是一个基于 OpenAI ChatGPT 的 Telegram Bot crate，当 bot 被 @ 提及时会使用 ChatGPT 来回答问题。

## 功能

- 检测 bot 被 @ 提及的消息
- 使用 OpenAI ChatGPT API 回答问题
- 支持流式响应，每 5 秒更新一次回复内容
- 自动配置支持自定义 API 地址和 API Key

## 使用方法

### 环境变量配置

创建 `.env` 文件并配置以下变量：

```env
TELEGRAM_BOT_TOKEN=your_telegram_bot_token_here
TELEGRAM_BOT_USERNAME=your_bot_username_here
OPENAI_API_KEY=your_openai_api_key_here
OPENAI_BASE_URL=https://api.openai.com/v1
```

### 运行示例

默认使用流式响应：

```bash
cargo run -p telegram-bot-ai
```

### 在代码中使用

#### 非流式响应

```rust
use telegram_bot_ai::TelegramBotAI;

let bot_ai = TelegramBotAI::new(
    bot_username.to_string(),
    openai_client::OpenAIClient::new(api_key.to_string())
);

// 在消息处理中
bot_ai.handle_message(bot, msg).await?;
```

#### 流式响应

```rust
use telegram_bot_ai::TelegramBotAI;

let bot_ai = TelegramBotAI::new(
    bot_username.to_string(),
    openai_client::OpenAIClient::new(api_key.to_string())
);

// 在消息处理中使用流式响应
bot_ai.handle_message_stream(bot, msg).await?;
```

## 配置说明

- `TELEGRAM_BOT_TOKEN`: 从 @BotFather 获取的 bot token
- `TELEGRAM_BOT_USERNAME`: 你的 Telegram bot 用户名（不带 @）
- `OPENAI_API_KEY`: OpenAI API 密钥
- `OPENAI_BASE_URL`: OpenAI API 地址（可选，默认为 https://api.openai.com/v1）
