# 图像生成功能使用指南

## 概述

Dbot 已集成文生图（Text-to-Image）功能，使用 OpenAI DALL-E API 生成图片。当用户发送包含特定关键词的消息时，机器人会自动识别并生成图片。

## 功能特性

- ✅ 自动识别图片生成请求（支持多种触发词）
- ✅ 使用 OpenAI DALL-E-3 模型生成高质量图片
- ✅ 支持自定义图片尺寸（1024x1024 默认）
- ✅ 自动发送生成的图片到 Telegram
- ✅ 完整的错误处理和日志记录

## 使用方法

### 触发方式

机器人会自动识别以下关键词来触发图片生成：

- `画` - 例如："画一只猫"
- `生成图片` - 例如："生成图片：美丽的风景"
- `生成图像` - 例如："生成图像：城市夜景"
- `画图` - 例如："请画图：一只小狗"
- `/image` - 例如："/image a sunset over mountains"
- `/draw` - 例如："/draw a cute robot"

### 使用示例

#### 示例 1：基础使用
```
用户：画一只可爱的小猫
机器人：[生成并发送图片]
```

#### 示例 2：详细描述
```
用户：生成图片：一座现代化的城市，夜晚，霓虹灯闪烁，未来感十足
机器人：[生成并发送图片]
```

#### 示例 3：使用命令
```
用户：/image a beautiful sunset over the ocean
机器人：[生成并发送图片]
```

#### 示例 4：@ 提及机器人
```
用户：@your_bot 画一幅山水画
机器人：[生成并发送图片]
```

## 技术实现

### 架构组件

1. **image-generation-client** (`crates/image-generation-client`)
   - OpenAI DALL-E API 客户端封装
   - 支持 DALL-E-2 和 DALL-E-3 模型
   - 支持自定义图片尺寸

2. **image-handlers** (`image-handlers`)
   - 图片生成请求检测和处理
   - 提取图片描述（prompt）
   - 调用图片生成客户端
   - 发送图片到 Telegram

3. **集成点** (`telegram-bot/src/runner.rs`)
   - 在 HandlerChain 中优先处理图片生成请求
   - 图片生成 Handler 在 LLM Handler 之前执行

### 处理流程

```
用户消息
  ↓
ImageGenerationHandler 检测触发词
  ↓
提取图片描述（prompt）
  ↓
发送"正在生成"提示
  ↓
调用 ImageGenerationClient.generate_image()
  ↓
获取图片 URL
  ↓
通过 Bot.send_photo() 发送图片
  ↓
返回 HandlerResponse::Stop（停止链处理）
```

## 配置说明

### 当前配置

图像生成功能使用与 LLM 相同的 OpenAI API Key 和 Base URL：

- `OPENAI_API_KEY` - OpenAI API Key（必需）
- `OPENAI_BASE_URL` - OpenAI API Base URL（默认：`https://api.openai.com/v1`）

### 默认设置

- **模型**：`dall-e-3`
- **图片尺寸**：`1024x1024`
- **响应格式**：URL（返回图片 URL）

### 代码中的配置

当前实现中，图像生成客户端使用以下默认配置：

```rust
ImageGenerationClient::with_base_url(api_key, base_url)
    // 默认模型：dall-e-3
    // 默认尺寸：1024x1024
```

## 扩展配置（可选）

如果需要支持自定义图像生成模型和尺寸，可以添加以下配置项：

### 1. 在 `BotConfig` 中添加配置字段

```rust
pub struct BotConfig {
    // ... 现有字段 ...
    
    /// 图像生成模型：dall-e-2 或 dall-e-3。环境变量：`IMAGE_GENERATION_MODEL`，默认 dall-e-3。
    pub image_generation_model: String,
    
    /// 图像生成尺寸。环境变量：`IMAGE_GENERATION_SIZE`，默认 1024x1024。
    /// 选项：256x256, 512x512, 1024x1024（dall-e-2）；1024x1024, 1792x1024, 1024x1792（dall-e-3）
    pub image_generation_size: String,
}
```

### 2. 在 `runner.rs` 中使用配置

```rust
let image_client = Arc::new(
    ImageGenerationClient::with_base_url(
        config.openai_api_key.clone(),
        config.openai_base_url.clone(),
    )
    .with_model(config.image_generation_model.clone())
    .with_size(parse_image_size(&config.image_generation_size))
);
```

## 错误处理

### 常见错误

1. **API Key 无效**
   - 错误信息：401 Unauthorized
   - 解决：检查 `OPENAI_API_KEY` 是否正确

2. **网络错误**
   - 错误信息：Network error
   - 解决：检查网络连接和 `OPENAI_BASE_URL` 配置

3. **图片描述为空**
   - 错误信息：No valid prompt extracted
   - 解决：确保消息中包含有效的图片描述

4. **API 配额不足**
   - 错误信息：Rate limit exceeded
   - 解决：检查 OpenAI API 配额

### 用户友好的错误消息

- `MSG_SEND_FAILED`: "抱歉，发送图片时出错。"
- `MSG_GENERATION_FAILED`: "抱歉，图片生成失败，请稍后重试。"
- `MSG_INVALID_PROMPT`: "请输入有效的图片描述。"

## 日志记录

图像生成过程会记录以下日志：

- **请求日志**：模型、尺寸、prompt 预览、API Key（脱敏）
- **请求 JSON**：完整的 API 请求体
- **响应日志**：生成的图片 URL
- **错误日志**：完整的错误链

查看日志：
```bash
tail -f logs/telegram-bot.log | grep -i image
```

## 测试

### 单元测试

```bash
# 运行图像生成 Handler 测试
cargo test -p image-handlers

# 运行图像生成客户端测试（需要 API Key）
OPENAI_API_KEY=your_key cargo test -p image-generation-client -- --ignored
```

### 集成测试

图像生成功能已集成到主机器人流程中，启动机器人后可以直接测试：

```bash
# 启动机器人
./target/release/dbot run

# 在 Telegram 中发送测试消息
# "画一只猫"
# "生成图片：美丽的风景"
```

## 成本说明

### OpenAI DALL-E 定价（参考）

- **DALL-E-3**
  - 1024x1024: $0.040 / 图片
  - 1024x1792 或 1792x1024: $0.080 / 图片

- **DALL-E-2**
  - 1024x1024: $0.020 / 图片
  - 512x512: $0.018 / 图片
  - 256x256: $0.016 / 图片

> 注意：实际价格请参考 [OpenAI 官方定价页面](https://openai.com/api/pricing/)

## 最佳实践

1. **清晰的描述**：提供详细、具体的图片描述可以获得更好的结果
2. **合理使用**：注意 API 调用成本，避免频繁生成
3. **错误处理**：监控日志，及时发现和解决问题
4. **用户引导**：可以在机器人帮助信息中说明如何使用图片生成功能

## 未来扩展

可能的改进方向：

1. **支持更多模型**：集成其他文生图模型（如 Stable Diffusion、Midjourney API）
2. **图片编辑**：支持图片编辑和变体生成
3. **批量生成**：支持一次生成多张图片
4. **图片缓存**：缓存生成的图片，避免重复生成
5. **用户配额**：为不同用户设置生成配额限制
6. **图片存储**：可选择将图片保存到本地或云存储

## 相关文档

- [部署指南](../DEPLOYMENT.md) - 机器人部署说明
- [README.md](../README.md) - 项目概述
- [OpenAI DALL-E API 文档](https://platform.openai.com/docs/guides/images) - 官方 API 文档
