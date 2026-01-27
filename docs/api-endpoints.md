# API Endpoints

## Anthropic (Claude)

公司: Anthropic PBC

## 智谱AI (Zhipu AI)

API地址: `https://open.bigmodel.cn/api/paas/v4/chat/completions`

### API 格式说明

智谱AI的API兼容 **OpenAI 格式**，而不是 Anthropic 格式。

## OpenAI 兼容格式（智谱AI）

### 端点地址
```
POST https://open.bigmodel.cn/api/paas/v4/chat/completions
```

### 请求格式
```json
{
  "model": "glm-4",
  "messages": [
    {
      "role": "user",
      "content": "你好"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 1024,
  "stream": false
}
```

### 请求头
```
Authorization: Bearer YOUR_API_KEY
Content-Type: application/json
```

## Anthropic API 格式 (仅供参考，智谱AI不兼容此格式)

### 端点地址
```
POST https://api.anthropic.com/v1/messages
```

### 请求头
```
x-api-key: YOUR_ANTHROPIC_API_KEY
anthropic-version: 2023-06-01
Content-Type: application/json
```

### 请求格式
```json
{
  "model": "claude-3-5-sonnet-20241022",
  "max_tokens": 1024,
  "messages": [
    {
      "role": "user",
      "content": "你好"
    }
  ]
}
```

### 响应格式
```json
{
  "id": "chat-123456",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "glm-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "你好！有什么可以帮助你的吗？"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 20,
    "total_tokens": 30
  }
}
```

### 支持的模型
- `glm-4`
- `glm-4-flash`
- `glm-4-air`
- `glm-3-turbo`

### 环境变量
```bash
export ZHIPU_API_KEY="your_api_key_here"
export ZHIPU_API_URL="https://open.bigmodel.cn/api/paas/v4/chat/completions"
```
